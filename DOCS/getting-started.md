# Getting Started with Winuxsh

A short walkthrough from zero to a working prompt with git status.

## 1. Build or download

```sh
git clone https://github.com/unixwin/winuxsh.git
cd winuxsh
cargo build --release
```

After building, the binary is at `target\release\winuxsh.exe`. You can run it
directly, or add `target\release` to your user `PATH` using your normal Windows
environment settings:

```sh
target/release/winuxsh.exe
```

If you are using the release zip, winuxsh automatically runs the activation
script on first start when command links are missing:

```bash
winuxsh winuxcmd/activate-winuxcmd.sh
```

That creates local command links inside `winuxcmd/`, so `ls`, `cat`, and
friends resolve normally. Once the links exist, startup skips activation.

## 2. Start the shell

```sh
winuxsh
```

You should see something like:

```text
user@DESKTOP C:\Users\you
%
```

Type `exit` or press Ctrl+D to quit.

## 3. See the git prompt

`cd` into any git repository:

```sh
cd C:\Users\you\repo
# if inside a repo, the prompt changes:
user@DESKTOP C:\Users\you\repo  git:(main) ●1 ✚2 ?1
%
```

Symbols at a glance:

| Symbol | Meaning |
|--------|---------|
| `●N`   | N files staged for commit |
| `✚N`   | N files modified but unstaged |
| `?N`   | N untracked files |
| `↑N`   | N commits ahead of upstream |
| `↓N`   | N commits behind upstream |
| `⚑N`   | N stashes saved |
| `✖N`   | N merge conflicts |

The branch name is green when the tree is clean, yellow when dirty.

## 4. Try some commands

```bash
pwd                                  # prints C:/Users/you/repo
ls -la                               # Unix-style listing
echo "hello from $USER"
for i in 1 2 3; do echo $i; done
if [ -f Cargo.toml ]; then echo "yep"; fi
cat Cargo.toml | grep name
grep -n "fn main" src/main.rs
```

Windows paths work directly:

```bash
ls C:\Windows\System32\drivers\etc
ls D:/Projects
cd "C:\Program Files"
```

Multiline blocks work naturally:

```bash
for f in *.toml; do
  echo "found $f"
done
```

## 5. Try git completions

```bash
git ad<Tab>                # completes to `git add`
git commit -<Tab>           # shows flags: --message, --all, --amend
git push --fo<Tab>          # completes to --force
git branch -<Tab>           # shows -d, -D, -m, -v, -a, -r
```

## 6. Set up your config

Create `~/.winshrc.toml` for structured settings:

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {git_prompt} {symbol}"

[editor]
edit_mode = "vi"

[completions]
matching = "prefix"

[plugins]
# Future plugin control plane. Plugin state, permissions, bundle versions, and
# managed updates belong here; shell code belongs in ~/.winshrc.
enabled = true
bundles = ["oh-my-winuxsh"]
load = ["git", "prompts", "keybindings"]

[winuxcmd]
# auto-detected from PATH; override only if needed
# enabled = true
# path = "D:/tools/winuxcmd/winuxcmd.exe"
# Set enabled = false if you prefer uutils, GNU, BusyBox, or another
# coreutils directory already on PATH.
```

Create `~/.winshrc` for interactive shell code:

```bash
export EDITOR=vim
alias ll='ls -la'
alias la='ls -a'
alias gst='git status'
alias gco='git checkout'
alias gl='git log --oneline --graph --decorate --all'

hello() {
  echo "hello from winuxsh"
}
```

`~/.winshrc` is sourced only for the interactive REPL. It does not run for
`winuxsh -c ...`, script files, or stdin script execution, so agent and CI
surfaces stay deterministic.

Use TOML for structured control-plane state such as plugins, permissions,
prompt settings, history, menus, and bundle updates. Use rc for free-form
rubash/bash-like shell code.

## 6b. Segment-based prompt (experimental)

Enable the p10k-style segment engine by adding to `~/.winshrc.toml`:

```toml
[shell]
prompt_style = "segments"
segment_preset = "classic"   # lean | classic | rainbow | pure | robbyrussell
```

The "classic" preset shows the directory in blue with a powerline triangle
separator, git status in green, and time/status on the right side. Each preset
has its own colour palette and element order.

Custom element order (overrides the preset):

```toml
left_prompt_elements = ["dir", "vcs", "newline", "prompt_char"]
right_prompt_elements = ["time", "status"]
```

Available elements: `dir`, `vcs`, `status`, `time`, `prompt_char`, `os_icon`,
`context` (user@host), `background_jobs`, `cmd_exec_time`, `newline`.

Restart winuxsh to pick up the changes.

## 7. Import your .zshrc (optional)

If you already have a `.zshrc` with Oh My Zsh, let winuxsh inspect it:

```sh
winuxsh --zsh-compat-report
winuxsh --zsh-compat-import-plan
```

Review the plan. If it looks safe (it scans, does not blindly source):

```sh
winuxsh --zsh-compat-import-apply
winuxsh --zsh-compat-doctor
```

## 8. Official plugin bundle

Winuxsh has a built-in plugin system. `oh-my-winuxsh` is the
official bundled plugin distribution, not an Oh My Zsh fork and not zsh plugin
support. It ships first-party packs such as `git`, `docker`, `kubectl`,
`npm`, `zoxide`, `direnv`, `dotenv`, `fzf`, prompt presets, and keybinding
presets.

The canonical TOML shape is:

```toml
[plugins]
enabled = true
bundles = ["oh-my-winuxsh"]
load = ["git", "prompts", "keybindings"]

[plugins.git]
enabled = true
permissions = ["cwd:read", "process:run:git"]

[plugins.zoxide]
enabled = false
permissions = ["cwd:read", "process:run:zoxide"]
```

Existing `[zsh.native_plugins]` and `[zsh.native_widgets]` config remains a
legacy migration compatibility surface, but new config should use `[plugins]`.
Use `winuxsh plugin list`, `winuxsh plugin search`, `winuxsh plugin themes`,
and `winuxsh plugin review` for current inventory, theme sources, and
permission checks; legacy `--zsh-native-packs` remains migration-only.

## What next

- [Zsh Migration Guide](zsh-migration-guide.md) for detailed `.zshrc` import
- [Plugin System Direction](plugin-system-direction.md) for the v3 plugin model
- [Plugin System Roadmap](plugin-system-roadmap.md) for the execution sequence
- [Oh My Winuxsh Bundle Plan](oh-my-winuxsh-bundle-plan.md) for the official bundle
- [Roadmap](winuxsh-roadmap.md) to see what is planned
- Source at [github.com/unixwin/winuxsh](https://github.com/unixwin/winuxsh)
