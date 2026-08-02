---
tags: [winuxsh, roadmap, v2]
created: 2026-07-13
status: active
---

# Winuxsh Roadmap

> Windows 原生、无隔离、给人和 agent 使用的 bash/zsh-like 终端
> 核心公式: winuxsh = rubash (shell 引擎) + winuxcmd.exe (coreutils) + reedline (REPL)

## 已完成 (v2 重写)

- [x] rewrite/v2-rubash 分支落地,单 commit c7e2c3c (+1514/-17817)
- [x] rubash 作为 lib 依赖,不再自实现 lexer/parser/ast/builtins
- [x] winuxcmd 通过 PATH 注入集成,不依赖 FFI/DLL
- [x] 补全系统 (TOML + bash 自动导入 + 三级缓存 + ListMenu)
- [x] 主题系统 (4 内置主题: default/dark/light/colorful)
- [x] Ctrl+C Win32 处理
- [x] REPL (reedline + 历史文件 .winuxsh_history)
- [x] 上游 PR unixwin/rubash#5 合入 (Windows PATH 大小写修复)
- [x] cargo build 零警告,14/14 测试通过
- [x] architecture.md、v2-plan.md 落盘 vault

## 当前方向更新 (2026-07-30)

- Zsh / Oh My Zsh 兼容层进入迁移与维护模式：保留 scanner、import-plan、
  诊断和一次性 onboarding，但不再把 zsh 插件、ZLE 或 Oh My Zsh 兼容作为
  插件系统身份。
- v3 主线转向 Winuxsh 自己的内置插件系统：`oh-my-winuxsh` 是官方 bundled
  plugin distribution，随 winuxsh 发行并支持独立更新/回滚。
- 插件实现顺序改为：先用 `kind = "builtin"` registry 接管现有 first-party packs，
  再做 process bridge，最后做 WASM/WASI 第三方运行时。
- rubash 始终跟随 `unixwin/rubash` `master` 最新版本；shell 语义缺口优先在
  rubash 上游修复，再由 winuxsh 更新依赖验证。

## 短期 - v2.1

### CI 基础设施
- [x] .github/workflows/ci.yml - push/PR 自动 cargo build + cargo test (PR #9 合入)
- [x] Windows 平台 (当前仅支持 Windows)
- [x] cargo fmt --check lint 步骤

### 兼容性测试套件
- [x] tests/compat/ 目录, .sh + .expected 配对
- [x] 覆盖: 变量展开、命令替换、管道、if/for/case/function、别名、exit code、echo flags (heredoc 待 T-4)
- [x] 通过 cargo test --test compat -- --ignored 运行 (10 个 fixture)

### master 合并
- [x] PR #9: rewrite/v2-rubash -> master, 已 squash 合并到 master (commit a50638f)
        修复了 cargo fmt --check 与 Cargo.lock tracked 问题，CI 全绿

### 脚本执行改进
- [x] T-4: execute_script 整体 tokenize+parse+execute，支持 heredoc / continuation / 多行 if/for (commit 792416f)

## 中期 - v2.2

### 工作方式
- [x] 先做 Nushell / 现代 Windows shell reference audit，仅参考设计，不引入 Nushell 依赖，不 vendor 外部源码
- [ ] 每个功能阶段先更新 Markdown 计划，再小步实现、测试、提交（v2.2 实施中）
- [x] Obsidian vault 中维护 `winuxsh/` 文件夹作为项目长期记忆
- [x] Nushell reference audit 落盘: `docs/planning/nushell-reference-audit.md`
- [x] zsh / Oh My Zsh / zsh 插件 reference audit 落盘: `docs/planning/zsh-reference-audit.md`
- [x] zsh-first 功能定位与现代 shell reference map 落盘: `docs/planning/winuxsh-positioning-and-feature-map.md`
- [x] Windows 原生 agent/user terminal 下一步计划落盘: `docs/planning/winuxsh-next-development-plan.md`
- [x] zsh 配置与插件兼容计划落盘: `docs/planning/zsh-compatibility-plan.md`
- [x] zsh 兼容接口可行性审计落盘: `docs/planning/zsh-compatibility-interface-audit.md`
- [x] Phase 0 hygiene: 清理误建的空 `--help` 目录，保留 `.tmp/` 未跟踪

### 补全系统增强
- [x] Phase 1 baseline: 修复 completion integration test stale API (`load_completion_dirs`)
- [x] Phase 2 foundation: 内置 `ls` / `grep` / `find` 默认补全定义
- [x] Phase 3 expansion: 内置 `cat` / `cp` / `mv` / `rm` / `mkdir` / `touch` / `chmod` 默认补全定义
- [ ] 扩充默认 TOML 补全定义的命令覆盖范围
- [ ] bash 自动导入覆盖更复杂的 complete 调用模式
- [ ] 补全三级缓存的 TTL/失效策略

### 配置一致性
- [x] Phase 5 config: [winuxcmd].path 参与 PATH injection

### 用户体验 (v2.2)
- [x] 引导式配置向导 (首次运行交互式问答, 自动生成 ~/.winshrc.toml)
- [x] {time}/{time_24} prompt 模板变量 (右侧时间提示)
- [x] 5 种 prompt 预设样式 (minimal/classic/powerline/multiline + right prompt options)
- [x] Unicode 提示符号预设: 设置向导可选 ❯ λ ▶ $ % 等符号
- [x] 配置化 prompt_symbol: config TOML + ShellConfig + WinuxshPrompt 全链路
- [x] 内置 40+ oh-my-zsh 风格 git 别名 (gst, gco, gp, gl, gd 等)
- [x] 补全系统增强: PATH 命令缓存 + 空 Tab 显示常用命令列表
- [x] 已修复: cd .. 不改变进程 cwd 的 bug
- [x] 已修复: /c/Users 路径格式不被 winuxcmd 识别的 bug
- [x] 已修复: C:\ 反斜杠路径被 tokenizer 吃掉的 bug

- [x] Vi 模式 (reedline 原生支持,主要工作量在键位配置)
- [x] Ctrl+R 历史搜索 (reedline 原生)
- [ ] 更多 prompt 自定义模板
- [x] Phase 6 themes: 用户自定义主题加载 (从 ~/.winuxsh/themes/)
- [x] zsh compat report CLI: 先输出扫描报告，不自动修改启动行为
- [x] zsh profile scanner/apply 第一层: `[zsh].auto_apply` 安全导入 `.zshrc` env/PATH/alias
- [x] Oh My Zsh layout importer Phase 2a: 静态 `_cmd` / `#compdef` / `_arguments` completion 资产翻译
- [x] zsh plugin tier importer Phase 2b: 插件分层报告 completion-only / alias-only / native-needed / unsupported
- [x] 原生 autosuggestions Phase 4a: 参考 zsh-autosuggestions，用 reedline history hinter 实现
- [x] 原生 syntax highlighting Phase 5a: 参考 zsh-syntax-highlighting main highlighter，用 reedline 实现
- [x] zsh prompt/theme compatibility Phase 6a: 扫描 `PROMPT` / `RPROMPT` 与简单 Oh My Zsh theme，翻译为 native prompt template
- [x] zsh Git prompt compatibility Phase 6b: 将 `$(git_prompt_info)` 桥接到 native `{git_prompt}` / `.git/HEAD` 渲染

## 长期 - v3

### 插件框架
- [x] v3 design doc opened: winuxsh-v3-plan.md
- [x] 插件 manifest schema：`kind = "builtin" | "wasm" | "process"` 已用于 registry/bundle manifest
- [x] Winuxsh plugin registry：现有 first-party builtin packs 已注册为官方 packs
- [x] `[plugins]` TOML 控制面：已解析 enablement、permissions、bundles、load；managed block/apply 待做
- [x] 插件 CLI：`plugin list/info/search/review/doctor/install/uninstall` 与 `plan enable/disable`、`enable/disable`、`update/rollback` 已接入
- [x] 完整执行路线：plugin-system-roadmap.md
- [x] `oh-my-winuxsh` bundled distribution：随 release 内置，支持独立版本更新
- [x] Phase 8 WASM command host：command modules 已支持 sha256 校验、memory cap、fuel timeout、exit code
- [x] Phase 14 WASM host IO ABI：`winuxsh:plugin/host` 支持受限 stdout/stderr 写入，缺失 memory / 越界 / 超限返回 `-1`
- [x] Phase 15 WASM command args ABI：`arg_count/arg_len/arg_read` 支持显式读取简单命令参数，非法 index / 缺失 memory / 越界返回 `-1`
- [x] Phase 16 WASM cwd read ABI：`cwd_len/cwd_read` 在 manifest 声明 `cwd:read` 后暴露 shell-visible `PWD`，无权限 / 缺失 memory / 越界返回 `-1`
- [ ] WASI/component host 与第三方长期运行时：后续扩展 completion/prompt/transform 能力
- [x] process/IPC 插件 bridge（外部工具适配与调试后端）：显式 opt-in、权限、timeout、command/hook fixture 已验证

### Oh-My-Winuxsh
- [x] 重建 `unixwin/oh-my-winuxsh`：本地保留 legacy state，当前分支改为官方 Winuxsh plugin bundle
- [x] `bundle.toml` 与 first-party `packs/*/plugin.toml`
- [x] 同步路线：`oh-my-winuxsh/docs/roadmap.md`
- [x] bundle baseline 随 winuxsh release 打包
- [x] `~/.winuxsh/plugin-lock.toml` 记录 bundle 版本、checksum、active/rollback path（本地 release artifact 已接入）
- [x] `winuxsh plugin update oh-my-winuxsh --from <path>` 独立安装/切换官方 bundle
- [x] `winuxsh plugin update oh-my-winuxsh --github-release latest|vX.Y.Z` 下载官方 release zip 与 `.sha256` 后进入同一校验/切换路径
- [x] Phase 9 discovery/review/doctor：`plugin search/review/doctor` 覆盖 active official inventory、权限审计、缺失 binary 和 drift 诊断
- [x] Phase 9 install/authoring：`plugin install/uninstall` 写入 managed `[plugins]`；oh-my bundle 提供 index、templates、authoring docs 和 CI gate
- [x] Phase 6 first-party assets：alias、completion、prompt preset、keybinding metadata 已由官方 bundle 接管，runtime 保留 compiled fallback
- [x] Phase 11 theme pack foundation：官方 bundle 可声明/校验/发布 theme assets，runtime 可从 active bundle 加载非内置主题
- [x] Phase 13 theme market discovery：`winuxsh plugin themes [--json]` 只读列出 built-in / user / active bundle 主题来源，为后续第三方主题分发保留产品层
- [x] Phase 14-17 WASM host ABI：oh-my 文档声明 stdout/stderr、simple argv、permission-gated cwd/env read 是当前 WASM public contract，WASI/component/shell mutation 仍未开放
- [x] Phase 7a import-plan CLI: `--zsh-compat-import-plan` 输出可审阅 `.winshrc.toml` patch，不自动写用户配置
- [x] Phase 7b import-apply CLI: `--zsh-compat-import-apply` 显式写入 `.winshrc.toml`，写前备份，仅替换 winuxsh 管理块
- [x] Phase 7c import-status CLI: `--zsh-compat-import-status` 只读检查 managed block / TOML / 备份 / 下一次 apply 可行性
- [x] Phase 7d rollback-plan CLI: `--zsh-compat-import-rollback-plan` 只读输出最近备份与恢复命令
- [x] Phase 7e doctor CLI: `--zsh-compat-doctor` 聚合 scan/status/rollback，给出安全 apply 判断和下一步命令
- [x] Phase 8a legacy native pack: `plugins=(git)` 缺少 OMZ 插件目录时提供保守 git alias pack，不覆盖用户 alias；后续迁移到 `oh-my-winuxsh/git`
- [x] Phase 8b legacy native pack: `plugins=(docker)` 缺少 OMZ 插件目录时提供保守 docker alias pack，不覆盖用户 alias；后续迁移到 `oh-my-winuxsh/docker`
- [x] Phase 8c dynamic completion scan: 识别 `tool completion zsh` 这类动态 completion generator，报告为 native provider 待接入
- [x] Phase 8d dynamic completion translation: 用注入 runner 将 `tool completion zsh` 输出翻译为 winuxsh `CommandDef`，尚不在启动时执行外部命令
- [x] Phase 8e dynamic completion runner: 显式 allowlist + timeout 执行动态 generator，默认不运行外部命令
- [x] Phase 9 dynamic completion provider: `[zsh.dynamic_completions]` 配置、磁盘缓存、启动接入，默认关闭
- [x] Phase 10a kubectl preset: `plugins=(kubectl)` 缺少 OMZ 目录时提供 native alias pack + disabled dynamic completion preset
- [x] Phase 10b npm preset: `plugins=(npm)` 缺少 OMZ 目录时提供安全 npm alias pack，并标记 F2/ZLE toggle 为 native UX 待实现
- [x] Phase 10c dynamic plugin shape scan: 区分 `script_generator` 与 `runtime_provider`，并标记 ZLE/hook/autoload 这类动态插件机制
- [x] Phase 11a runtime completion provider: `[zsh.runtime_completions]` 显式 allowlist + timeout，在 Tab 时接入 npm-style `completion -- "${words[@]}"` 动态候选
- [x] Phase 12a native lifecycle hooks: `[hooks]` 支持 `precmd` / `preexec` / `chpwd` REPL hook surface，不 source zsh 函数体
- [x] Phase 12b native hook suggestions: 扫描 `add-zsh-hook` / `*_functions` / hook 函数定义，输出可审阅 `[hooks]` TODO，不自动执行
- [x] Phase 13a keybinding migration suggestions: 扫描 `zle -N` / custom `bindkey`，输出可审阅 native reedline keybinding TODO；不支持 ZLE runtime
- [x] Phase 14a keybinding presets: 旧 `[zsh.native_widgets]` 兼容读取后，将 recognized autosuggest/history keybinding 名称映射到 reedline event
- [x] Phase 14b native UX plugin presets: 缺少插件目录时也将 `zsh-autosuggestions` / `zsh-history-substring-search` / syntax-highlighting 类插件归为 native UX
- [x] Phase 15a autoload/function suggestions: 扫描 `autoload` 与函数定义，按 completion/hook/widget/prompt/helper 形态输出报告和 import-plan TODO
- [x] Phase 16a native dynamic plugin preset: `direnv` 通过旧显式 opt-in，在 native precmd/chpwd hook 点运行 `direnv export bash`
- [x] Phase 16b native dynamic plugin preset: `alias-finder` 通过旧显式 opt-in，在 native preexec hook 点提示已知 alias
- [x] Phase 16c native dynamic plugin preset: `zoxide` 通过旧显式 opt-in，提供 native `z` command shim 并用 lifecycle hook 记录目录
- [x] Phase 16d native dynamic plugin preset: `thefuck` 通过旧显式 opt-in，提供 native `fuck` correction shim，基于上一条交互命令调用 `thefuck`
- [x] Phase 16e native dynamic plugin preset: `command-not-found` 通过旧显式 opt-in，在命令缺失时输出 Windows-native 安装搜索提示
- [x] Phase 16f native selector plugin preset: `fzf` / `zsh-interactive-cd` 通过旧显式 opt-in，提供 native `cdf` / `fzf-cd` 目录选择 shim
- [x] Phase 16g native state plugin preset: `last-working-dir` 通过旧显式 opt-in，提供 native `lwd` 与交互 REPL 启动目录恢复
- [x] Phase 16h native env plugin preset: `dotenv` 通过旧显式 opt-in，安全解析当前目录 `.env` 并写入 rubash env
- [x] Windows-native host contract stabilization: `cd` 后同步 rubash `PWD` 与 process cwd，`pwd` 默认显示 `C:/...`，winuxcmd 路径参数兼容旧 `/c/...` 输入，空输入/前缀命令补全恢复
- [x] Phase 18 completion probe: 新增非交互 `--completion-probe` 入口，覆盖空 Tab、前缀命令、PATH/PATHEXT、管道后命令位与参数位不误补全
- [x] Phase 19 blank argument path completion: 修复 `cd <Tab>` / `ls <Tab>` 这类空参数位不返回当前目录候选的问题
- [x] Phase 20 path completion polish: 保留目录前缀、转义空格路径、隐藏文件按 `.` 前缀显示、目录优先排序
- [x] Phase 21 shell-word-aware completion: 补全切词理解反斜杠转义和简单引号，修复 `two\ w` / `"two w` 这类路径补全
- [x] Windows cwd authority regression: 启动时以真实 process cwd 初始化 rubash `PWD`，且 `cd target; native-child` 同一交互行中同步 process cwd，避免 prompt/ls 与 `PWD` 分裂
- [x] Phase 22 prompt indicator polish: `[shell]` 支持 emacs/vi/default/multiline/history-search prompt indicators，补齐 zsh-like 模式提示入口
- [x] Phase 23 history config polish: `[history]` 支持 history path、max size、ignore-space-prefixed，保持默认 `~/.winuxsh_history`
- [x] Phase 24 completion UX config: `[completions]` 支持 case sensitivity、prefix/substring matching、max command results
- [x] Phase 25 menu UX config: `[menus]` 支持 completion/history page size 与 max entry lines
- [x] Phase 26 zsh-style keybinding name subset: 常见 `bindkey KEY action-name` 映射到 reedline 原生事件；不执行 ZLE 函数体
- [x] Phase 27 native Windows path literals: 裸 `C:\...` 输入在 rubash tokenization 前规范化为 `C:/...`，避免反斜杠被 bash 词法当作转义符吞掉
- [x] Phase 28 interactive multiline collector: REPL 识别未完成的 `if/for/while/case/function` 等复合命令块，显示 PS2/continuation prompt，完整后一次性交给 rubash script execution
- [x] Phase 29 bash smoke fixture: 将用户手工 20 段 bash/zsh-like smoke 脚本整理为可持续 compat fixture，优先覆盖条件判断、循环、函数、重定向、路径与 exit status
- [x] Phase 30 rubash AND/OR status semantics: 修复 `false && a || b` / `[ ... ] && a || b` 这类 AND/OR list 跳过语义，保持 shell 语义在 rubash，不在 winuxsh 重建执行器
- [x] Phase 31 legacy native pack manifest: 列出现有 git/docker/kubectl/npm/keybinding/lifecycle packs，并提供只读旧 CLI inventory (`--zsh-native-packs` / `--zsh-native-packs-json`)；后续迁移到 `winuxsh plugin list`
- [x] Phase 31b legacy cleanup: 旧 CLI inventory 保留为迁移兼容入口，用户文档和帮助文案迁移到 `winuxsh plugin ...`
- [x] Phase 32 zsh-lite profile plan: 基于现有 `[zsh]` / `[zsh.native_widgets]` / `[zsh.native_plugins]` 生成可审阅默认 zsh-like 配置块
- [x] Phase 33 Git daily-use polish: `git <Tab>` / 子命令 / flag 补全已接入并测试，README 补齐 alias、completion、prompt 文档，让 git 插件成为第一等 daily shell 能力
- [x] Phase 33a oh-my-zsh-style git prompt status: 新增 `crates/winuxsh-runtime/src/git_status.rs` 通过 `git status --porcelain -b` / rev-list / stash list 收集 branch/dirty/staged/unstaged/untracked/deleted/ahead/behind/stashes/conflicts；prompt 模板新增 `{git_dirty}` / `{git_staged}` / `{git_unstaged}` / `{git_untracked}` / `{git_deleted}` / `{git_ahead}` / `{git_behind}` / `{git_stashes}` / `{git_conflicts}` / `{git_status}` 紧凑串；theme 新增 `git_clean` / `git_dirty` / `git_status_detail` 着色；`{git_prompt}` 默认形如 `git:(main) ●2 ↑1 ↓1 ?3`，clean=green / dirty=yellow；`completions/defaults/git.toml` 内置 add/commit/push/pull/checkout/switch/branch/merge/rebase/reset/restore/stash/status/log/diff/init/clone 子命令补全
- [x] Phase 34 p10k-style segment-based prompt engine: new prompt_segments.rs module with 5 presets (lean/classic/rainbow/pure/robbyrussell), powerline separators, multiline prefixes
- [x] README / tutorial documentation baseline: README.md / README-zh.md 重写为用户入口，新增 `docs/src/zsh-migration-guide.md` 迁移教程
- [x] Plugin system direction refresh: `docs/planning/plugin-system-direction.md` 改为 Winuxsh-native plugin system + bundled `oh-my-winuxsh`
- [x] Oh My Winuxsh bundle plan: `docs/planning/oh-my-winuxsh-bundle-plan.md` 定义重建、bundle、更新、lockfile 和迁移策略
- [ ] Plugin registry implementation: builtin packs first, then process bridge, then WASM/WASI
- [ ] zsh/Oh My Zsh 兼容导入层维护：只修 bug、保安全导入，不继续扩大为 zsh runtime 或 zsh plugin support

### Rubash 能力验证
- [x] Rubash/bash 能力矩阵：新增 `docs/src/rubash-bash-compat-matrix.md`，按 compat fixtures、host contract、本地 GNU Bash upstream gate 分层记录已验证能力和缺口
- [x] Winuxsh host GNU Bash upstream gate (2026-07-28): 新增
      `scripts/run-bash-upstream-with-winuxsh.sh`，shell under test 指向
      `winuxsh/target/debug/winuxsh.exe`，结果 86 total / 86 pass / 0 fail，
      summary 位于
      `target/bash-upstream-tests/summary.md`；
      本地执行说明见 `docs/planning/bash-upstream-local.md`，不纳入默认 CI，也不 vendor
      Bash upstream tests
- [x] Phase 17 host contract matrix: 为 winuxsh host 层补充 PATH/env/cwd/home/stdout/stderr/exit-code 二进制级集成测试
- [x] Phase 18 completion probe tests: 通过 `winuxsh --completion-probe` 验证真实 Shell 初始化后的 REPL 补全候选
- [x] Phase 19 path completion tests: 覆盖空参数位路径补全，并保持管道后空命令位仍补命令
- [x] Phase 20 path polish tests: 覆盖 `src/ma` 不丢前缀、空格文件名转义、隐藏文件过滤和目录优先排序
- [x] Phase 21 shell word tests: 覆盖转义空格匹配、引号内路径匹配、补全替换 span 不截断 token
- [x] REPL cwd sequence tests: 覆盖 `execute_line("cd target; cwdprobe")` 中 Windows `.cmd` 子进程 cwd 与 `PWD` 一致
- [x] Phase 22 prompt indicator tests: 覆盖 emacs/vi insert/normal、多行提示、Ctrl+R history search passing/failing 模板
- [x] Phase 23 history config tests: 覆盖默认 history、`~` 路径展开、max size、ignore-space-prefixed reedline 接入
- [x] Phase 24 completion UX tests: 覆盖默认 prefix、substring、case-sensitive path、command result cap
- [x] Phase 25 menu UX tests: 覆盖默认菜单配置、TOML 解析、zero fallback、reedline menu builder 接入
- [x] Phase 26 keybinding mapping tests: 覆盖常见 zsh-style keybinding 名称映射、import-plan 启用入口、unsupported diagnostics 降噪
- [x] Phase 27 native Windows path tests: 覆盖 `ls C:\...` 与 `cd C:\...; pwd` 的二进制级 host contract
- [x] Phase 28 multiline REPL tests: 覆盖 pending buffer 对 `if/fi`、`for/done`、函数体、引号、管道续行、反斜杠续行和注释行的完整性判断
- [x] Phase 29 bash smoke tests: 增加聚合 smoke fixture，并确保失败用例先拆成小回归修复后再纳入 smoke
- [x] Phase 30 AND/OR tests: 覆盖 `true &&`, `false &&`, `true ||`, `false ||`, 以及 `[ 1 -eq 2 ] && yes || no`
- [x] Phase 31 native pack inventory tests: 覆盖 pack registry text/json 输出，不改变启动行为
- [x] Phase 32 profile plan tests: 覆盖 `agent` / `zsh-lite` 生成 TOML 与 managed-block apply/status/rollback 兼容性
- [x] Phase 33 git pack tests: 覆盖 `git <Tab>`、常见子命令/flag 补全与用户 alias override
- [ ] 作业控制/内建命令语义优先走 rubash，不在 winuxsh 重复实现

## 关键架构决策 (锁定)

- License: GPL-3.0-or-later (与 rubash 一致,同 unixwin org)
- rubash 集成方式: git 依赖,非本地路径
- winuxcmd 集成方式: PATH 注入,非 FFI/DLL
- 配置文件: .winshrc.toml (保留向后兼容)
- 历史文件: .winuxsh_history
- rust-version: 1.70 (minimum)
- rubash 版本策略: 跟随 `unixwin/rubash` `master` 最新版本；更新 root `Cargo.lock` 后验证 winuxsh
- 插件框架: v3 以内置 Winuxsh plugin registry + bundled `oh-my-winuxsh` 为主线；
  先 `builtin`，再 process bridge，最后 WASM/WASI；zsh 只保留迁移/维护层

---

参见: architecture.md | plugin-system-direction.md | oh-my-winuxsh-bundle-plan.md | v2-plan.md | rubash-pr-windows-path.md | winuxsh-v2.2-reference-plan.md | winuxsh-v3-plan.md | winuxsh-positioning-and-feature-map.md | winuxsh-next-development-plan.md | zsh-reference-audit.md | zsh-compatibility-plan.md | zsh-compatibility-interface-audit.md | winuxsh-native-zsh-plugin-pack-plan.md | zsh-migration-guide.md
