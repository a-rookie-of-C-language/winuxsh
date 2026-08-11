# Winuxsh Configuration

Use this reference when configuring Winuxsh, isolating tests, or explaining the
startup/config surfaces. Keep user-facing edits small, reversible, and native
to Winuxsh.

Winuxsh uses three user-facing files:

- `~/.winuxshrc`: primary interactive startup file. Put theme selection,
  `WINUXSH_PLUGINS`, `WINUXSH_THEME_PLUGIN`, aliases, functions, exports, and
  framework sourcing here.
- `~/.winshrc`: legacy interactive fallback. It is sourced only when
  `~/.winuxshrc` is absent.
- `~/.winshrc.toml`: legacy/managed structured state for plugin CLI metadata,
  migration blocks, tests, package/update metadata, and advanced
  machine-editable overrides. Do not make it the normal human-authored startup
  file.

Direct project commands, `winuxsh -c`, script files, stdin script execution,
agent tests, and CI must stay independent from all interactive rc files. Use
`-C` / `--repl-command` when a one-shot probe specifically needs REPL startup
and lifecycle hooks.

## Minimal ~/.winuxshrc

```bash
WINUXSH_THEME=p10-classic
WINUXSH_THEME_PLUGIN=theme-p10-classic
WINUXSH_PROMPT_SYMBOL=">"
export WINUXSH_THEME WINUXSH_THEME_PLUGIN WINUXSH_PROMPT_SYMBOL

WINUXSH_PLUGINS=(prompt-core git)

if [ -z "${HOME:-}" ] && [ -n "${USERPROFILE:-}" ]; then
  HOME="$USERPROFILE"
  export HOME
fi

if [ -z "${WINUXSH:-}" ]; then
  for __winuxsh_bundle in "$HOME/.oh-my-winuxsh" "$HOME/.winuxsh/oh-my-winuxsh" "$HOME/.winuxsh/bundles/oh-my-winuxsh"/*; do
    if [ -f "$__winuxsh_bundle/oh-my-winuxsh.winux" ]; then
      WINUXSH="$__winuxsh_bundle"
      export WINUXSH
      break
    fi
  done
fi

[ -f "$WINUXSH/oh-my-winuxsh.winux" ] && . "$WINUXSH/oh-my-winuxsh.winux"
winuxsh_prompt_use_template "{cwd} {git_prompt}{prompt_char} " "{status}{time} " 2>/dev/null || true
```

Common changes:

- Change `WINUXSH_THEME_PLUGIN` to select an official theme plugin.
- Change `WINUXSH_PLUGINS=(...)` to enable bundled source plugins.
- Put user aliases, functions, and environment exports in `~/.winuxshrc`.
- Keep source plugins in bundles such as `~/.oh-my-winuxsh`; do not paste
  plugin framework internals into the user's rc.

Prompt, theme, git prompt, aliases, completions, and shell helper behavior
should be modeled as official or third-party plugins. Winuxsh core should
provide lifecycle APIs, native host integration, and safety boundaries only.
Missing official bundles or theme plugins should be diagnosed rather than
silently replaced with a different built-in theme.

## Home And Path Resolution

Treat `HOME` and `USERPROFILE` as Windows-host paths that may arrive in either
native form (`C:/Users/me`, `C:\Users\me`) or compatibility slash-drive form
(`/c/Users/me`). Prompt display, setup writes, plugin bundle paths, completion
caches, and tilde expansion should normalize those forms before comparing or
writing files. By default, prompt cwd display should render the home directory
as `~` and descendants as `~/path`; use `WINUXSH_PROMPT_CWD_STYLE=full` or
`basename` only when the user asks for that style.

## Managed TOML

Use `~/.winshrc.toml` only when the feature is deliberately structured or
machine-managed:

```toml
[editor]
edit_mode = "emacs"

[history]
max_size = 10000

[plugins]
enabled = true
bundles = ["oh-my-winuxsh"]
```

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

`~/.winuxshrc` and legacy `~/.winshrc` use Bash syntax:

```bash
export EDITOR=vim
alias gs='git status'
mkcd() { mkdir -p "$1" && cd "$1"; }
```

Do not rely on either file for tests, CI, or `winuxsh -c`. If command behavior
depends on an environment variable, pass it explicitly in the command or test
environment.

## Test Isolation

Use `WINUXSH_CONFIG` to point child-process probes at a temporary
`.winshrc.toml`:

```bash
WINUXSH_CONFIG=C:/Temp/winuxsh-test/.winshrc.toml target/debug/winuxsh.exe -c 'printf "%s\n" "$PWD"'
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
or completion internals. Translate safe intent into `~/.winuxshrc`, managed
TOML only when appropriate, or Winuxsh plugin suggestions.
