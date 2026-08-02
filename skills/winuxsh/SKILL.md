---
name: winuxsh
description: >-
  Run, script, debug, configure, update, or develop Winuxsh: a Windows-native,
  AI-native Bash-compatible shell backed by rubash and WinuxCmd. Use for
  winuxsh -c/-C invocation from PowerShell or agent hosts, Windows path and
  tilde behavior, ~/.winuxshrc and legacy managed config,
  plugin/bundle/WPM/self-update workflows, command-provider discovery, or
  repository verification for Winuxsh itself.
---
# Winuxsh

Treat Winuxsh as a native Windows process that provides a GNU Bash-compatible
shell and Unix-style command experience on Windows. Rubash owns Bash parsing and
execution; WinuxCmd supplies coreutils-style external commands through normal
Windows PATH resolution; Winuxsh owns the host integration, REPL, completion,
prompt, config, WPM, plugin, and update surfaces.

Do not model Winuxsh as WSL, MSYS2, Git Bash, Cygwin, PowerShell, zsh, or a
virtual Unix filesystem. There is no canonical `/usr/bin` layer to target.
Prefer native Windows paths and verify the active binary when behavior matters.

## Progressive Loading

- Start with only this file. Read references only for the active surface.
- Prefer live CLI discovery for dynamic facts: `winuxsh --help`,
  `winuxsh --version`, `winuxsh plugin --help`, JSON plugin commands,
  `command -v`, `wpm ...`, `git status`, and Cargo metadata.
- Reference routing:
  - `runner.md`: launch/quote `winuxsh -c`, `-C`, stdin, scripts, args, paths.
  - `development.md`: edit or test the Winuxsh repository.
  - `config.md`: `~/.winuxshrc`, legacy config, prompt/theme startup, tests.
  - `plugins.md`: self-update, WPM, plugin CLI, bundles, rollback, review.
  - `winuxcmd-discovery.md`: missing utilities, links, WPM, provider issues.
  - `bash-sh-syntax.md`: non-trivial Bash scripts or Bash behavior checks.
- If a CLI result conflicts with a reference, trust the target binary first and
  describe the version-specific behavior.

## First Moves

- From Codex/pwsh, run shell work through `winuxsh -c '...'`; pwsh may launch
  the process, but Bash syntax belongs inside the single-quoted program.
- When the `-c` program contains quotes, backslashes, `$`, `!`, or nested
  escapes, prefer a `.sh` script file (`winuxsh script.sh`) or stdin: pwsh
  argument handling can eat or mangle inline programs (a double-quoted value
  may arrive trimmed, split, or as a parse error). Inline `-c` is reliable
  only for simple commands.
- Choose `-c` for deterministic quiet script/CI/agent execution: no rc, no
  prompt, exact exit codes. Choose `-C` / `--repl-command` only when one-shot
  REPL startup state or lifecycle hooks are required, and verify support with
  `winuxsh --help`.
- If already inside Winuxsh, write Bash directly. Nest `winuxsh -c` only when
  a fresh child shell is part of the test.
- Prefer native Windows paths: `C:/Users/me/project` for durable scripts,
  quoted `C:\Users\me\project` when testing backslashes, and `/c/...` only for
  compatibility probes.
- Discover active utility providers instead of assuming `/usr/bin`.
- When editing this repository, read `AGENTS.md` and
  `references/development.md` before changing code or tests. Keep unrelated
  worktree changes untouched.

## Operating Rules

- Use Winuxsh as the command language for Winuxsh work. Do not let pwsh own
  pipes, expansion, redirects, globs, or Bash control flow.
- In pwsh, single-quote the `-c` program. Use script files or stdin when
  quoting becomes hard to audit.
- If a command misbehaves only when launched from pwsh (arguments eaten,
  trimmed, or merged), reproduce it through a script file before debugging
  Winuxsh itself; the host shell is the usual culprit.
- Do not substitute PowerShell, Python, Node, or awk for shell behavior or
  file-edit glue that should be exercised in Winuxsh. Use Winuxsh plus
  WinuxCmd/coreutils; use WPM when utilities or command links are missing.
- For generated edits, write temp output first; never redirect over original
  source, config, or skill files.
- Keep `winuxsh -c`, script files, and stdin execution quiet and deterministic:
  no banners, stable stdout/stderr, and exact exit-code propagation.
- Treat `~/.winuxshrc` as the primary interactive user entry point for theme
  selection, plugin lists, aliases, functions, exports, and source-framework
  startup. `~/.winshrc` is only a legacy fallback when `~/.winuxshrc` is absent.
- Treat `~/.winshrc.toml` as legacy/managed structured state for plugin CLI
  metadata, migrations, tests, and advanced machine-editable overrides, not as
  the default user-authored startup file.
- Keep `winuxsh -c`, scripts, stdin execution, agent tests, and CI independent
  from any interactive rc file.
- Keep prompt, theme, git prompt, aliases, completions, and shell helpers in
  bundled source plugins when possible; core owns host APIs and boundaries.
- Use newer docs and source code over old roadmap notes. Zsh-related surfaces
  are legacy migration/importer material unless the user explicitly asks about
  that migration path.

## Avoid

- Do not present current Winuxsh work as zsh compatibility work. Keep zsh notes
  short and migration-only.
- Do not hardcode local machine paths, installed user names, per-user install
  directories, or one developer's PATH into open-source skill text.
- Do not hardcode static command inventories, plugin lists, utility lists, or
  release metadata that the active binary or package manager can report.
- Do not fix Bash language behavior in Winuxsh host code when the bug belongs
  in rubash, and do not reintroduce WinuxCmd FFI/DLL routing.
