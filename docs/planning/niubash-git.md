# niubash-git：无 MSYS 原生最小 Git（路线 B 可行性评估）

状态：**评估完成，待拍板动工**
日期：2026-09-16
前置讨论：niubash 打包 git vs git 打包 niubash → 结论 niubash 做载体；路线 A（收编 MinGit）保留为 fallback。

## 1. 目标

在 Windows 上发行一个**不带任何 MSYS2/POSIX 环境的原生 git**：

- 产物只有 git 本体（exe + libexec + templates），零 bash、零 perl、零 MSYS2 runtime
- git 需要跑 sh 的场景（hooks、`git-mergetool` 等 16 个脚本）由 **niubash 的 sh shim** 从 PATH 承接
- git 不再自带 shell = "shell 即平台"叙事的落地：装 niubash 才是完整体验

## 2. 实测依据（本机 Git for Windows 2.55.0(3)）

- `git.exe`（mingw64 版）导入表：全 Windows 系统 DLL + 4 个自带小 DLL（libiconv-2 / libintl-8 / libpcre2-8-0 / zlib1）。**无 msys-2.0.dll**。git 本体是纯原生 PE，MSYS2 只是发行版拖的行李。
- mingw64 原生层合计 **63 MB**：bin 61M（exe 19 个 ~20M，DLL 41M 主要是 openssl/curl 全家桶）、libexec 198K。
- `libexec/git-core` 非二进制残留仅 **16 个文件**：`git-mergetool`(+`--helper`/`--lib`/`mergetools/`)、`git-filter-branch`、`git-submodule`(shim)、`git-subtree`、`git-request-pull`、`git-merge-{octopus,one-file,resolve}`、`git-sh-setup`、`git-sh-i18n`、`git-web--browse`、`git-quiltimport`、`git-update`。
- `git.exe` 本体 4.2M。

## 3. 依赖边界（git 上游 INSTALL 原文确认）

| 依赖 | 状态 | 处理 |
|---|---|---|
| zlib | **唯一硬依赖**（won't build without it） | vcpkg 静态链接 |
| POSIX sh | 跑 bisect/request-pull/mergetool 等脚本 | **niubash sh shim 承接**（本方案核心） |
| ssh | clone/push over ssh | 系统 `System32\OpenSSH\ssh.exe`（Win10+ 自带），无需捆绑 |
| libcurl ≥7.61 | http/https/imap-send | 保留，TLS 用 **schannel**（系统证书库，连 ca-bundle 都省了；git 官方支持 `http.sslBackend=schannel`） |
| iconv | reencode 编码转换 | 可 NO_ICONV；建议先静态带上（~2M），丢编码功能不值 |
| perl 5.26+ | 仅 send-email / git svn | **NO_PERL 砍**，没人用 |
| expat | 仅 http-push（WebDAV） | **NO_EXPAT 砍** |
| Tcl/Tk | gitk / git-gui | **NO_TCLTK 砍** |
| gettext | i18n | NO_GETTEXT（英文界面）；中文文案不属于 git 本体痛点 |
| python | 仅 git-p4 | 不装 |
| SHA1/SHA256 | — | 内置 SHA1DC + SHA256_BLK，`NO_OPENSSL`，无 openssl 依赖 |
| PCRE2 | 仅 `grep -P` | 不带，丢 `grep -P` 可接受（GNU grep 在 winuxcmd 有） |

**砍完剩什么**：zlib（静态）+ libcurl/schannel（静态或单 DLL ~1.5M）+ 可选 iconv（静态）。加 git.exe ~4.5M。

## 4. 工具链：推荐 MSVC + CMake + vcpkg

两条可行路线对比：

| | **MSVC + CMake + vcpkg（推荐）** | MSYS2 mingw-w64 gcc（官方发行版路线） |
|---|---|---|
| 上游维护度 | `contrib/buildsystems/CMakeLists.txt` + 上游 CI **`vs-build` job 每日跑**（checkout microsoft/vcpkg 到 `compat/vcbuild/vcpkg`） | git-for-windows 官方产物路线，最成熟 |
| 与 niubash 构建体系 | 完全贴合：`scripts/build.py` + `build-with-vs.ps1`（VS/MSVC），vcvars 环境复用 | 构建机需装 MSYS2，产物体系另一套 |
| CRT | `/MT` 静态 CRT → 单 exe，零运行时依赖 | 默认静态 gcc runtime |
| **ARM64** | CMake 脚本内建 x64/**arm64**/x86 分支，VS 工具链直接交叉编译，**匹配 niubash 双架构发布线** | MSYS2 clang-aarch64 可用但生态偏年轻 |
| 构建过程对 sh 的需求 | CMake 配置阶段需要 sh（构建机有 Git Bash 即可，不进产物） | 天然在 MSYS2 里 |
| 体积控制 | /MT + /GL + 可 MinSizeRel，最细粒度 | 也可控，但 objcopy/strip 工具链另一套 |

CMake 路线要点（读 `contrib/buildsystems/CMakeLists.txt` 确认）：

- Windows 默认 `USE_VCPKG=ON`：找不到 `compat/vcbuild/vcpkg` 时自动 `vcpkg_install.bat` 拉依赖；也可 `NO_VCPKG` 自管
- `find_package(ZLIB REQUIRED)`；CURL/EXPAT/Iconv/Intl/PCRE2 自动探测，缺了自动定义 `NO_CURL`/`NO_EXPAT`/`NO_ICONV`/`NO_GETTEXT`
- MSVC 分支完整（`/std:c11`、`wmainCRTStartup`、`invalidcontinue.obj` 等）
- `RUNTIME_PREFIX` 默认开启 → 产物可便携化，`GIT_EXEC_PATH` 相对 exe 定位，适合 niubash 目录结构
- `SKIP_DASHED_BUILT_INS=ON` 可跳过 libexec 硬链接式薄壳，进一步瘦身
- 注意：该文件自我定位 contrib 级（有 TODO：gitk/git-gui/gitweb、Windows NLS），但上游 CI 在跑 → 不是死代码，缺口风险可控

## 5. 产物形态与体积预估

```
git/                        # niubash 安装目录下，例如 AppData\Local\Programs\Niubash\git\
├── bin/git.exe             # ~4.5M，/MT 静态 CRT，唯一入口
├── libexec/git-core/       # git-remote-https 等少量 exe + 16 个 sh 脚本(保留,由 niubash sh 跑)
├── share/git-core/templates/
└── etc/                    # （schannel 用系统证书库，可能整个不需要）
```

- 预估 **12–18 MB**（对比：MinGit zip 37M / 解压 ~130M / 官方 Git 安装包 ~300M 磁盘占用）
- 极限模式（NO_ICONV、SKIP_DASHED_BUILT_INS、curl 静态、/Os）：奔着 <10M

## 6. sh 层承接设计

- git 在 Windows 上 spawn `sh` 走 PATH 搜索；niubash 安装期落盘的 `sh.exe` shim 正好被找到
- 16 个 git 官方脚本 + 典型社区 sh hooks 作为测试用例进 rubash 的 **gnu-compat harness**（`tests/gnu-compat/run-83.sh` 同款流程），跑不过的 gap 进 GAPS.md 按"差异化→文档→清池→批修"处理
- 降级策略：无 niubash 环境裸跑此 git 时，sh hooks 缺失 → git 会报 `sh: command not found`；在打包层加一次性 `git config` 提示或 wrapper 检测（PoC 阶段先不管，属打磨项）
- `bisect`（上游明说依赖 sh）由 niubash 承接后应完整可用——这本身是 harness 的一个高价值用例

## 7. winuxcmd / awk 侧

- git 运行时**不依赖 awk**（构建走 CMake 也不碰 Makefile 的 awk 规则）
- 那 16 个脚本中个别用了 awk/sed（如 `git-filter-branch`）→ 由 wpm 清单模式预装 gawk 兜底（json/txt 清单已确认可用，非阻塞项）
- winuxcmd 当前缺 awk/gawk 不影响本方案 PoC；中期待 winuxcmd 自带 awk 后移除预装

## 8. 集成

- **WPM 包**（`wpm install niubash-git`）：解包到安装目录、生成命令链接（只暴露 `git.exe` + `gitk` 无）、PATH 注入指向 `git/bin`；独立于 shell 层更新，符合三层更新体系
- 安装器可选组件打勾，安装包体积 +~15M 可接受
- PATH 顺序：本包目录只有 git 自身文件，无 sh/find/awk 冲突面；`sh` 不由本包提供（这是特性）
- 凭据：`git-credential-wincred`（原生 exe，构建自带产出）

### 8.1 仓库拓扑（已定）：独立打包仓，非 fork 仓

新建 `unixwin/niubash-git` 仓库，三条边界划死：

1. **不 vendor git 源码**。CI 从 git-for-windows tag 拉源码 → 应用 `patches/` → 构建。源码永不进本仓，上游升级 = 改一个版本变量。仓内容仅：构建脚本（MSVC+CMake+vcpkg 封装，贴合 build.py 约定）、`vcpkg.json`、`patches/`（目标零或个位数）、CI workflow。
2. **资产出口单一，三个消费端共用一个 Release**：
   - Release 产物：`niubash-git-<ver>-x64.zip` / `-arm64.zip`（内含 wpm 所需的 manifest 元数据）
   - wpm 清单 → 指向该 Release（`wpm install git`）
   - niubash 安装器可选组件 → 拉该 Release URL（不内嵌，安装包体积不变）
   - 独立分发 zip → 同一个 Release 直链（引流入口）
   - 单一源头，杜绝版本漂移。
3. **品牌与合规**：仓/包名 `niubash-git`，zip 名带上游版本号；`git --version` 构建串加标识（如 `2.55.0.windows.3.niubash`）；GPLv2：保留版权、附源码获取链接。不使用 "Git for Windows" 名称。

同步节奏：安全补丁随时跟（git 上游 security release 必须快速出包）；例行版本季度跟进。不放 niubash 主仓的原因：发版节奏耦合（git 安全包高频 vs niubash 功能版低频）+ release assets 混杂。

## 9. 风险清单

1. **CMake 系统为 contrib 级**：上游 CI 在用但官方产物走 MSYS2；git 大版本升级时 CMakeLists 可能有缺口要自己补（跟随成本：每季度例行版本 + 安全补丁）
2. **t/ 测试套件是 perl 驱动**：CMake 有 CTest 集成（单元测试 + t.suite 注册）但完整跑需要 perl；需自定义验收集（见 §10），不全套照搬
3. **NO_ICONV/NO_GETTEXT 功能损失**：非 UTF-8 仓库编码转换、git 本体中文界面缺失；先带 iconv 缓解前者
4. **sh shim 质量**：hooks/bisect/mergetool 的可用性完全押在 rubash POSIX 覆盖上——这既是风险也是本方案的验收驱动力
5. **schannel 证书行为差异**：企业自签证书场景与 openssl 行为不同（走系统证书库，多数场景反而更顺）
6. **向后兼容**：不改变 niubash 现有安装/更新路径；git 是新增独立包，无存量影响

## 10. 分阶段计划与验收

**Phase 0 — PoC（预计 2–4 天）✅ 2026-09-16 完成**
- [x] clone git 上游 master（或 git-for-windows tag），走 CMake + VS + vcpkg 构建通过（git-for-windows v2.55.0.windows.2，VS18/CMake4.4，`USE_VCPKG=OFF` + classic vcpkg 直链）
- [x] vcpkg 依赖定稿（PoC 口径）：zlib + curl(**schannel**，TLS 走系统 crypt32/bcrypt，零 openssl) + iconv，3 个运行时 DLL（iconv-2/zlib1/libcurl）
- [x] 产物落地：`git --version` 2.55.0；init/add/commit/log/status 全通
- [x] https clone（github.com octocat/Hello-World）走 schannel 验证通过，含**零配置开箱**场景（GIT_CONFIG_GLOBAL 置空）
- [x] 构建脚本 `scripts/build.py`（相对路径、tarball 拉源码→configure→build→collect→zip）
- 产物：**dist 20MB / zip 20MB（MinGit zip 37MB）**，布局 = MinGit 同款（`mingw64/bin`、根级 `etc/`）
- 额外验证：sh hooks 经 PATH 承接机制实证通过（pre-commit 由 PATH 上的 sh 执行）
- 关键编译开关：`USE_VCPKG=OFF`（上游文档写的 `NO_VCPKG` 是无效变量——文档 bug，PR 候选）、`SKIP_DASHED_BUILT_INS=ON`（省 ~180 个 3.9MB 硬链接壳，zip 从 309MB 回落 20MB）、`BUILD_TESTING=OFF`（clar 单测在干净构建有生成顺序问题）
- 已知坑（PoC 实测）：contrib CMake 的 perl-gen custom command 在 msbuild 下静默失败，build.py 已用绝对路径重生成兜底；`http.sslBackend` 若用户全局 config 残留 `openssl`（官方 Git 安装器写入的机器很常见）会 die——单后端构建应忽略不兼容 backend 配置，**patch 候选 #1**
- 仓名定案：**niu-git**（`unixwin/niu-git`，本地仓已建 D:\repo\niu-git，远程建仓待用户）

**Phase 1 — 打包**
- [ ] wpm 包（json 清单）+ 命令链接 + PATH 注入
- [ ] arm64 交叉构建产物
- [ ] 安装器可选组件

**Phase 2 — sh 承接验收**
- [ ] 16 个官方脚本过 gnu-compat harness，gap 入账
- [ ] pre-commit/post-checkout 等典型 sh hooks 实测
- [ ] `git bisect` 全流程实测
- [ ] 无 niubash 环境的降级提示

**退出条件**：PoC 失败（MSVC 编不过且 2 周内修不动）→ 回退路线 A（收编 MinGit 裁包），前期工作大部分可复用（打包/集成层完全一致）。

## 11. Open questions

1. ~~git 版本基线~~ → 已定建议：跟 git-for-windows tag（Windows 侧 regression 修复同步，MSVC 路线不依赖其 MSYS2 部分）
2. ~~包名~~ → 已定：`niubash-git`（独立打包仓 `unixwin/niubash-git`，wpm 包名 `git`，见 §8.1）
3. ~~独立裸 git zip~~ → 已定：出，且与 wpm 包/安装器组件共用同一 Release（见 §8.1）
4. 待定：仓名最终确认（`niubash-git` vs `git-portable` 类中性名，影响独立分发的品牌观感）
5. 待定：独立 zip 裸跑（无 niubash）时 sh hooks 缺失的引导文案与提示方式
