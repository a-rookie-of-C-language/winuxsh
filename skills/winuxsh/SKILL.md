---
name: winuxsh
description: >-
  Native Winuxsh session guidance for Codex and other agents when the active
  terminal is already Winuxsh. Use for Winuxsh repository work, .winuxshrc
  edits, oh-my-winuxsh bundles, .winux scripts, command installation, PATH
  setup, shell debugging, WinuxCmd/WPM discovery, and Windows-native
  Bash-compatible command execution without wrapping ordinary commands in
  winuxsh -c.
---
# Winuxsh Native Session

Assume the current command environment is already Winuxsh unless the user or
runtime explicitly says it is PowerShell, cmd.exe, WSL, Git Bash, MSYS2, or
Cygwin. Run ordinary commands directly in the current Winuxsh session.

Do not wrap normal work in `winuxsh -c`, `winuxsh.exe -c`, `pwsh`, PowerShell,
`cmd.exe /c`, `bash.exe`, WSL, Git Bash, MSYS2, or Cygwin. Use `winuxsh -c`
only when the task deliberately tests a fresh child shell, script-mode
semantics, or a command launched from a non-Winuxsh host.

## Core Rules

- Treat Winuxsh as a native Windows process with GNU Bash-compatible syntax,
  Windows PATH resolution, and WinuxCmd command links for Unix-style tools.
- Prefer Windows-native paths such as `C:/Users/me/project`; avoid `/mnt/c`,
  MSYS/Cygwin roots, Git Bash assumptions, and hardcoded local user paths.
- Use Bash-compatible syntax directly: pipes, redirects, variables, arrays,
  command substitution, globs, functions, aliases, and conditionals.
- Search with `rg` or `rg --files` first. Use available WinuxCmd/coreutils
  commands directly; if common tools are missing, diagnose WinuxCmd command
  links instead of switching shells.
- Keep non-interactive command paths quiet and deterministic: no banners,
  stable stdout/stderr, and exact exit-code propagation.
- Preserve unrelated worktree changes. Do not revert user edits or generated
  state that is outside the requested change.

## Permission And Failure Handling

- A failed command is not automatically a code or shell bug. First classify the
  failure: missing binary, bad cwd, quoting/parser issue, PATH/provider issue,
  sandbox restriction, Windows access denial, or command semantics.
- If `cd`, directory traversal, file writes, installs, link rebuilds, package
  updates, or tests that write outside the workspace fail with access denied,
  read-only filesystem, sandbox, sharing violation, or locked-file symptoms,
  treat it as a likely permission/escalation issue before changing code.
- For permission-looking failures, retry with the approved escalation mechanism
  when the environment provides one and the action is necessary. If escalation
  is unavailable or unsafe, report the exact blocked path/action and the
  command that needs approval.
- Do not hide real failures with `|| true` except for explicit optional probes
  such as `command -v tool || true`.

## Project Work

- Read `AGENTS.md` before editing this repository. If `AGENTS.md` still
  mentions `winuxsh -c`, apply it only to external host launchers; inside a
  native Winuxsh tool session, run project commands directly.
- Rubash owns shell language semantics. Winuxsh owns Windows host integration,
  REPL, completion, prompt, config, WPM, plugin, and update surfaces. WinuxCmd
  owns external Unix-style utilities and command links.
- Use `~/.winuxshrc` as the primary interactive user entry point. Keep
  `~/.winshrc` as legacy fallback and `~/.winshrc.toml` as legacy/managed
  structured state, not the default human-authored config surface.
- Keep prompt, theme, git prompt, aliases, completions, and shell helpers in
  bundled source plugins when possible. Core should provide host APIs and
  safety boundaries.

## Progressive Loading

Start with this file. Read references only for the active surface:

- `references/development.md`: repository edits and test selection.
- `references/config.md`: `.winuxshrc`, legacy config, prompt/theme startup.
- `references/plugins.md`: WPM, plugin CLI, bundle updates, permissions.
- `references/winuxcmd-discovery.md`: missing utilities, command links, WPM.
- `references/bash-sh-syntax.md`: non-trivial Bash behavior or scripts.
- `references/runner.md`: only when launching Winuxsh from a non-Winuxsh host.

If live CLI output conflicts with a reference, trust the active binary first
and call out the version-specific behavior.
