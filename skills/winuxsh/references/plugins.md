# Winuxsh Updates, Plugins, And Packages

Use this reference for shell self-update, WPM package management, WinuxCmd
updates, `winuxsh plugin ...`, `oh-my-winuxsh` bundle updates, permission
review, and rollback. Keep the three update planes distinct.

## Update Planes

- `winuxsh --self-update`: update Winuxsh itself by downloading and launching
  the latest installer through native Windows networking.
- `wpm update winuxcmd`: update WinuxCmd packages and command links inside the
  selected WinuxCmd root.
- `winuxsh plugin update oh-my-winuxsh ...`: update the official bundled
  Winuxsh plugin distribution independently from shell and command packages.

Bundled official plugins are not shell-core builtins. Prompt templates, theme
plugins, git prompt helpers, aliases, completions, and shell helper functions
should live in source plugin packs under bundles such as `oh-my-winuxsh`.
Winuxsh core should provide lifecycle hooks, host APIs, native helpers, and
safety boundaries. Missing official bundles or theme plugins should fail
visibly instead of silently selecting a hidden built-in substitute.

## Winuxsh Self-Update

```bash
winuxsh --self-update --check
winuxsh --self-update --dry-run
winuxsh --self-update
```

Use `--check` before changing a user's installation. Use `--dry-run` when the
user wants download validation without running the installer.

## WPM Packages And WinuxCmd Updates

```bash
cmd_dir=$(dirname "$(command -v winuxcmd.exe)")
winuxcmd.exe wpm index status --root "$cmd_dir"
winuxcmd.exe wpm search jq --root "$cmd_dir"
winuxcmd.exe wpm info jq --root "$cmd_dir"
winuxcmd.exe wpm install jq --root "$cmd_dir"
winuxcmd.exe wpm update winuxcmd --root "$cmd_dir"
winuxcmd.exe wpm links rebuild --root "$cmd_dir" --force
```

Use WPM rather than telling agents to work around missing utilities with pwsh,
Python, Node, awk, or ad hoc scripts.

## Plugin CLI

```bash
winuxsh plugin list --json
winuxsh plugin info git --json
winuxsh plugin bundle status --json
winuxsh plugin plan enable git --json
winuxsh plugin enable git
winuxsh plugin plan disable git --json
winuxsh plugin disable git
```

Use `plan` before writing managed state. If the target build supports review or
doctor commands, use them before enabling plugins that execute external
commands or require permissions. For interactive Oh My-style source plugins,
prefer editing `WINUXSH_PLUGINS=(...)` and `WINUXSH_THEME_PLUGIN=...` in
`~/.winuxshrc`; the CLI-managed TOML path is for structured metadata,
migrations, tests, and advanced overrides.

## Bundle Updates

Official bundle updates target `oh-my-winuxsh`:

```bash
winuxsh plugin update oh-my-winuxsh --from dist/oh-my-winuxsh-1.0.0.zip --checksum-file dist/oh-my-winuxsh-1.0.0.zip.sha256
winuxsh plugin update oh-my-winuxsh --github-release latest
winuxsh plugin rollback oh-my-winuxsh
```

Use only one of `--from` or `--github-release`. Use only one of `--checksum` or
`--checksum-file`. GitHub release updates verify the release checksum through
the supported command path; do not add a second checksum source unless the CLI
requires it.

## Config Model

Human-authored interactive plugin state lives in `~/.winuxshrc`:

```bash
WINUXSH_THEME_PLUGIN=theme-p10-classic
WINUXSH_PLUGINS=(prompt-core git common-aliases)
```

Managed plugin state may still live under `[plugins]` in `~/.winshrc.toml`.
`winuxsh plugin enable` and `disable` can write managed config and create
backups when the target build supports it, but do not use TOML as the default
Oh My-style startup surface.

Prompt/theme ownership should flow through plugins:

- `prompt-core` owns prompt lifecycle functions, prompt templates, and git
  snapshot handoff variables such as `WINUXSH_PROMPT_GIT`.
- `theme-*` packs own visual presets and color choices.
- `git` owns aliases and lightweight Git helper functions, but live prompt
  rendering should be coordinated through prompt/theme plugins to avoid late
  redraw churn.

Git prompt status should follow the Powerlevel10k/gitstatus shape: prompt
rendering consumes the latest coherent host snapshot, while Winuxsh keeps a
persistent `--gitstatus-daemon` helper and cache warm in the background. Late
Git work must warm a later prompt, not repaint the active input line. Dirty
compact markers should use the active theme's dirty style, not generic
white/gray detail text.

Developer/test overrides:

- `WINUXSH_PLUGIN_BUNDLE_PATH`
- `WINUXSH_PLUGIN_BUNDLE_ROOT`
- `WINUXSH_APP_BUNDLE_PATH`
- `WINUXSH_PLUGIN_LOCK`

Use these only for tests or local development. Legacy importers may still exist
for onboarding, but they are not the current plugin identity and should not
drive new docs or examples.
