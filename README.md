# Winuxsh

[中文](README-zh.md) · English

> **bash for Windows — no WSL, no MSYS2, no PowerShell surprises.**
> Built for humans and coding agents, tested against the bash spec.

```text
me@DESKTOP C:\Users\me\repo\winuxsh  master ●2 ✚1 ?3
❯
```

Branch name, dirty state, staged/untracked counts — all built into the prompt
the moment you `cd` into a git repo.  No plugins to install.  No config to
tweak.  Just `❯` and go.

## Why

You know how PowerShell does `ls` and gives you a table of objects?  Or how
`test -f Cargo.toml` doesn't exist in pwsh?  Or how your coding agent keeps
failing because the shell it expected isn't there?

Winuxsh fixes that.  It is the terminal that feels like bash, lives on
Windows like PowerShell, and speaks Windows-native paths (`C:\Users`, not
`/mnt/c/Users`).

```bash
# That works — immediately, from the first keystroke
cd C:\Users\me\Documents
ls -la
git status
if [ -d repo ]; then echo "found it"; fi
```

## Quick start

```pwsh
cargo build --release
target\release\winuxsh.exe
```

The first time you start it, you get a setup wizard (think `oh-my-zsh` install):

```text
🎉  Welcome to Winuxsh 0.6.0!
✨  A bash-compatible shell for Windows — no WSL, no MSYS2 required.

  Let's get you set up.  (Press Enter to accept defaults.)

📝  Editing mode
  │  emacs = standard keybindings (Ctrl+A/E/K, Tab, Ctrl+R)
  │  vi    = vim-style insert/normal modes
  │  Enter choice [emacs/vi]:

🎨  Colour theme
  │  Enter choice [default/dark/light/colorful]:

🎵  Prompt symbol
  │  ❯ heavy right-pointing angle (powerlevel10k style)
  │  λ lambda (functional/minimal)
  │  $ dollar sign (classic bash)
  │  % percent sign (classic fish)
  │  Enter choice [❯/λ/$/%]:

⏱️  Right-side info
  │  off  = no right prompt
  │  time = show current time (HH:MM)
  │  full = time + git branch
  │  Enter choice [off/time/full]:

🔄  Show git branch/status in the prompt [Y/n]:
```

That is it.  One round of questions, `~/.winshrc.toml` is written, and
every launch after that is instant.

## What makes it different

| You want this                  | PowerShell gives you       | Winuxsh gives you          |
|--------------------------------|----------------------------|----------------------------|
| `ls` / `grep` / `find` / `cp`  | Aliases + cmdlets          | Real `winuxcmd` coreutils |
| `if [ -f file ]; then`         | `if (Test-Path file) {`    | Real bash `if`            |
| `for i in a b c; do ... done`  | `foreach ($i in ...) {`    | Real bash `for`           |
| `git:(master) ●2 ✚1` in prompt | Have to install oh-my-posh | Built in, works on `cd`   |
| `gst` / `gco` / `gp` (git)     | Custom aliases             | Pre-installed git aliases |
| `$(command)`                   | `$(command)` but different | Same as bash              |
| `exit 127`                     | `$LASTEXITCODE`            | Same as bash              |
| `C:\Users` paths               | Works natively             | Also works natively       |
| `Ctrl+R` history search        | Yes (but different)        | Yes (reedline, standard)  |
| `cd .. && pwd`                 | Yes                        | Yes                       |
| Setup wizard                   | No                         | Yes, oh-my-zsh style      |
| Coding agent friendly          | Not really                 | `-c` / `script.sh` quiet  |
| Reads your `.zshrc`            | No                         | `--zsh-compat-report`     |

## Screenshots

**A real terminal session** — `cd`, `ls`, `git status`, block completion:

```text
me@DESKTOP C:\Users\me\repo\winuxsh
❯ ls
Cargo.toml  src/  crates/  DOCS/  tests/  README.md

me@DESKTOP C:\Users\me\repo\winuxsh  master ●2 ✚1
❯ git status
Changes to be committed:
  modified:   src/shell.rs

me@DESKTOP C:\Users\me\repo\winuxsh  master ●2
❯ if [ -f Cargo.toml ]; then
  echo "yes, it is a rust project"
fi
yes, it is a rust project
```

**Autosuggestions** — ghost text after the cursor, accept with `Ctrl+Space`:

```text
me@DESKTOP C:\Users\me\repo\winuxsh  master
❯ cd rep○  ← "cd repo/" shows as hint
```

**Syntax highlighting** — commands in green, flags in cyan, errors in red.

**Right prompt** — time, git branch, or both:

```text
me@DESKTOP C:\Users\me\repo\winuxsh     09:47
❯
```

## What it runs

```
winuxsh = rubash (shell engine) + winuxcmd.exe (coreutils) + reedline (REPL)
```

| Component | Job |
|-----------|-----|
| `rubash`  | bash-compatible parser, executor, builtins, functions, heredocs |
| `winuxcmd`| Unix coreutils (`ls`, `cat`, `grep`, `find`, `cp`, `mv`, `rm`, ...) |
| `reedline`| Interactive editing, history, Tab completion, autosuggestions |
| `~/.winshrc.toml` | Structured settings — prompt, theme, editor, completion, plugins |
| `~/.winshrc` | REPL startup script — `export`, `alias`, functions, shell code |
| `.zshrc` scan | `--zsh-compat-report` reads your zsh intent → native TOML |

## For zsh / Oh My Zsh users

You don't need to migrate manually.  Winuxsh can inspect your existing setup
and propose a safe import:

```pwsh
winuxsh --zsh-compat-report         # see what is importable
winuxsh --zsh-compat-import-plan    # preview the TOML it would write
winuxsh --zsh-compat-import-apply   # write it (with backup)
winuxsh --zsh-compat-doctor         # overall health check
```

Things that get imported from `.zshrc`:
- `PATH` / `ENV` exports (safe subset — no expansion or backtick)
- `alias` declarations
- `PROMPT` / `RPROMPT` (translated to native TOML template)
- Oh My Zsh plugin intent (e.g. `git` → suggested Winuxsh `git` plugin)

What stays in `.zshrc` and continues working there:
- Complex `compdef` / `_arguments` (native completion reads the same files)
- Custom functions (winuxsh reads the same function source via rubash)
- Theme expressions that can't be translated (`%F{...}` parsing)

## Official Winuxsh plugins

Winuxsh includes a built-in plugin system. `oh-my-winuxsh` is the official
bundled plugin distribution for first-party packs. It is not an Oh My Zsh fork
and not zsh plugin support.

Release packages include a baseline copy at `bundles/oh-my-winuxsh` next to
`winuxsh.exe`; user-installed bundle updates in `%LOCALAPPDATA%` or the plugin
lock file override that baseline.

Useful entry points:

```sh
winuxsh plugin list
winuxsh plugin search git
winuxsh plugin themes
winuxsh plugin review git
winuxsh plugin install git
winuxsh plugin uninstall git
```

The first bundled packs cover:

- daily dev tools: `git`, `docker`, `kubectl`, `npm`;
- navigation and workflow: `zoxide`, `direnv`, `dotenv`, `fzf`;
- prompt and keybinding presets;
- command hints such as `command-not-found`.

Plugin state belongs in `~/.winshrc.toml`; user shell code belongs in
`~/.winshrc`. Current zsh compatibility commands remain migration tools, and
legacy `--zsh-native-packs` inventory remains migration-only.

## Git daily use

The official `git` plugin is on by default and makes Git feel first-class:

```text
gst  => git status        gco  => git checkout      gp   => git push
gl   => git pull          gd   => git diff          ga   => git add
gc   => git commit -v     gb   => git branch        gr   => git remote
gsta => git stash push    gstp => git stash pop     glg  => git log --stat
```

Completion is subcommand and flag aware:

```bash
git <Tab>                # add, commit, push, pull, checkout, ...
git a<Tab>               # add
git commit --<Tab>       # --message, --amend, --no-verify, ...
git push --force<Tab>    # --force / --force-with-lease
```

Prompt templates can include `{git_prompt}` for branch, dirty state, staged,
untracked, ahead/behind, stash, and conflict counts. Put personal aliases in
`~/.winshrc` with normal shell syntax; user aliases override plugin aliases.

## Configuration reference

Minimal `~/.winshrc.toml`:

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {git_prompt}{symbol}"
prompt_symbol = "❯"
right_prompt_format = "{time} "

[editor]
edit_mode = "emacs"           # emacs | vi

[theme]
current_theme = "default"     # default | dark | light | colorful | ocean | forest | cyberpunk | minimal | custom

[completions]
matching = "prefix"           # prefix | substring
case_sensitive = false
```

Interactive shell customizations go in `~/.winshrc`:

```bash
export EDITOR=vim
alias ll='ls -la'
alias gst='git status'

hello() {
  echo "hello from winuxsh"
}
```

Full reference with all options: [DOCS/getting-started.md](DOCS/getting-started.md).

## Project status

| Layer    | Status |
|----------|--------|
| rubash   | ✔ bash parser/executor — tracked by [Rubash Bash Compatibility Matrix](DOCS/rubash-bash-compat-matrix.md) and local upstream gate |
| winuxcmd | ✔ Unix coreutils via PATH injection, no FFI |
| REPL     | ✔ reedline: history, Tab, autosuggest, syntax highlight |
| Completion | ✔ Built-in (ls, grep, find, git…), TOML, bash import, cache |
| Git prompt | ✔ Non-blocking async refresh, configurable symbols |
| Setup wizard | ✔ First-run guided config |
| Zsh migration | ✔ Scanner and import plan; not a zsh plugin runtime |
| Plugin roadmap | ✔ Built-in Winuxsh registry plus bundled `oh-my-winuxsh` |
| User themes  | ✔ `~/.winuxsh/themes/<name>.toml`; official bundle themes include `ocean`, `forest`, `cyberpunk`, `minimal` |
| Vi mode      | ✔ reedline native |
| Ctrl+R       | ✔ reedline native |
| v3 roadmap   | Plugin framework, Oh-My-Winuxsh, job control |

## How to help

- Report a bug?  Open an issue.
- Want a feature?  Check [the roadmap](DOCS/winuxsh-roadmap.md).
- Build from source: `cargo build --release`.
- Installer releases add Winuxsh to the user PATH and add/update a Windows Terminal profile.
- Portable release zips remain available and include `winuxsh.exe`, `winuxcmd/winuxcmd.exe`, icon assets, `winuxcmd/activate-winuxcmd.sh`, and the bundled `oh-my-winuxsh` baseline.
- On first start, winuxsh runs the activation script once if command links are missing.
- Self-update: run `self-update` inside the REPL, or `winuxsh --self-update` outside it, to download and launch the latest installer via native WinHTTP.
- Run the tests: `cargo test`.

## Documentation

- [Getting Started](DOCS/getting-started.md) — full config reference
- [Installer and Self-Update](DOCS/installer.md)
- [Plugin System Direction](DOCS/plugin-system-direction.md)
- [Plugin System Roadmap](DOCS/plugin-system-roadmap.md)
- [Oh My Winuxsh Bundle Plan](DOCS/oh-my-winuxsh-bundle-plan.md)
- [Zsh Migration Guide](DOCS/zsh-migration-guide.md)
- [Roadmap](DOCS/winuxsh-roadmap.md)
- [Architecture](DOCS/architecture.md)

## License

GPL-3.0-or-later.  See [LICENSE](LICENSE).
