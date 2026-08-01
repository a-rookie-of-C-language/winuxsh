# Winuxsh Updates, Plugins, And Packages

Use this reference for shell self-update, WPM package management, WinuxCmd
updates, `winuxsh plugin ...`, `oh-my-winuxsh` bundle updates, permission
review, and rollback. Keep the three update planes distinct.

## Update Planes

- `winuxsh --self-update`: update Winuxsh itself by downloading and launching
  the latest installer through native Windows networking.
- `wpm update winuxcmd`: update WinuxCmd packages and command links inside the
  selected WinuxCmd root.
- `winuxsh plugin update oh-my-winuxsh ...`: update the official Winuxsh plugin
  bundle independently from shell and command packages.

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

Use `plan` before writing managed TOML. If the target build supports review or
doctor commands, use them before enabling plugins that execute external
commands or require permissions.

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

Canonical plugin state lives under `[plugins]` in `~/.winshrc.toml`.
`winuxsh plugin enable` and `disable` write managed config and create backups
when the target build supports it. Prefer the CLI over hand-editing user plugin
state.

Developer/test overrides:

- `WINUXSH_PLUGIN_BUNDLE_PATH`
- `WINUXSH_PLUGIN_BUNDLE_ROOT`
- `WINUXSH_APP_BUNDLE_PATH`
- `WINUXSH_PLUGIN_LOCK`

Use these only for tests or local development. Legacy importers may still exist
for onboarding, but they are not the current plugin identity and should not
drive new docs or examples.
