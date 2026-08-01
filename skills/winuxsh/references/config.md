# Winuxsh Configuration

Use this reference when configuring Winuxsh, isolating tests, or explaining the
two config surfaces. Keep user-facing edits small, reversible, and native to
Winuxsh.

Winuxsh uses two user files:

- `~/.winshrc.toml`: structured control plane for prompt, editor, history,
  completion, WinuxCmd, hooks, plugins, bundles, package/update metadata, and
  managed migration state.
- `~/.winshrc`: interactive REPL shell script for `export`, `alias`, functions,
  and shell code. It is sourced only for the interactive REPL, not for
  `winuxsh -c`, script files, stdin script execution, agent tests, or CI.

## Minimal TOML

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {git_prompt}{symbol}"
prompt_symbol = ">"
right_prompt_format = "{time} "

[editor]
edit_mode = "emacs"

[completions]
matching = "prefix"
case_sensitive = false

[plugins]
enabled = true
bundles = ["oh-my-winuxsh"]
```

Common changes:

- Set `edit_mode = "vi"` for vi-style editing.
- Set `matching = "substring"` for looser completions.
- Set `right_prompt_format = ""` to disable the right prompt.
- Put user aliases and functions in `~/.winshrc` unless the feature has a
  structured TOML field.

## WinuxCmd Override

Only add this when auto-discovery does not find the intended WinuxCmd:

```toml
[winuxcmd]
path = "D:/tools/winuxcmd/winuxcmd.exe"
```

If the user intentionally wants another utility provider to win through PATH:

```toml
[winuxcmd]
enabled = false
```

Do not disable WinuxCmd while debugging a release-style bundle whose command
links are expected to provide coreutils.

## Interactive Shell Code

`~/.winshrc` uses Bash syntax:

```bash
export EDITOR=vim
alias gs='git status'
mkcd() { mkdir -p "$1" && cd "$1"; }
```

Do not rely on this file for `winuxsh -c` or CI. If command-mode behavior
depends on an env var, pass it explicitly in the command or test environment.

## Test Isolation

Use `WINUXSH_CONFIG` to point probes at a temporary `.winshrc.toml`:

```bash
WINUXSH_CONFIG=C:/Temp/winuxsh-test/.winshrc.toml winuxsh -c 'printf "%s\n" "$PWD"'
```

Plugin and bundle tests may also use:

- `WINUXSH_PLUGIN_BUNDLE_PATH`
- `WINUXSH_PLUGIN_BUNDLE_ROOT`
- `WINUXSH_APP_BUNDLE_PATH`
- `WINUXSH_PLUGIN_LOCK`
- `WINUXSH_SKIP_WINUXCMD_ACTIVATION`

Treat these as developer/test overrides, not normal user setup.

## Legacy Importer Boundary

Legacy shell importers are for migration/onboarding only. They are not the
current configuration model, not the plugin system, and not runtime startup.
Do not source arbitrary legacy startup files, plugin scripts, editor widgets,
or completion internals. Translate safe intent into native TOML, `~/.winshrc`,
or Winuxsh plugin suggestions.
