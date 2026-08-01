---
name: winuxsh
description: >-
  Use when Codex needs to run, script, debug, or develop Winuxsh: a
  Windows-native GNU Bash-compatible shell and coreutils experience launched
  with winuxsh -c from pwsh/Codex or another host, using -C/--repl-command for one-shot REPL-style commands when the target build supports it; Bash pipelines, redirects,
  heredocs, functions, arrays, jobs, exit status, Windows-native paths such as
  C:/Users/me/project plus quoted drive-letter backslash paths, with /c
  compatibility inputs, WinuxCmd utility discovery, WPM package installs and
  WinuxCmd updates, ~/.winshrc.toml and ~/.winshrc configuration,
  winuxsh --self-update, plugin and bundle commands, or repository
  verification for winuxsh itself.
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
## First Moves
- When working from Codex/pwsh, run shell work through `winuxsh -c '...'` and
  put Bash syntax inside the single-quoted program. Read
  `references/runner.md` for quoting, Codex CLI/tool wrappers, args, stdin,
  and path rules.
- If a Codex tool displays a pwsh wrapper, make the launched command itself
  `winuxsh -c '...'` for Winuxsh validation or project work. Pwsh may start the
  process; it must not own the shell program.
- For one-shot command execution, choose intentionally: `-c` is the quiet
  script/CI path; `-C` / `--repl-command` is the emerging non-interactive REPL
  path that loads REPL startup state and lifecycle hooks before executing one
  REPL-style line. Verify `winuxsh --help` on the target binary before relying
  on `-C`, because older installed releases may not include it yet.
- If already inside Winuxsh, write Bash directly. Nest `winuxsh -c` only when a
  fresh child shell is part of the test.
- Prefer native Windows paths: `C:/Users/me/project` for durable scripts and
  examples, `C:\Users\me\project` when intentionally testing backslash input,
  and `/c/Users/me/project` only for compatibility probes.
- When a command needs `ls`, `grep`, `find`, `jq`, `7zip`, `yq`, `zstd`, or
  another utility, discover the active provider and WPM state instead of
  assuming `/usr/bin`. Read `references/winuxcmd-discovery.md`.
- When writing scripts, use Bash surfaces naturally: variables, command
  substitution, functions, arrays, case, loops, pipelines, redirects, heredocs,
  jobs, signals, and exit status. Read `references/bash-sh-syntax.md`.
- When changing config, packages, updates, plugins, or bundles, read
  `references/config.md` and `references/plugins.md`.
- When editing this repository, read `AGENTS.md` and
  `references/development.md` before changing code or tests.
## Operating Rules
- Use Winuxsh as the command language for Winuxsh work. Pwsh may be the host
  process that launches `winuxsh -c`, but it should not own pipes, expansion,
  redirects, globs, or Bash control flow.
- In pwsh, single-quote the `-c` program so `$x`, `$(...)`, `;`, `|`, `<`, and
  `>` reach Bash unchanged. Use script files or stdin for commands that become
  quote-heavy.
- Do not substitute PowerShell, Python, Node, or awk for shell behavior or
  file-edit glue that should be exercised in Winuxsh. Use Winuxsh plus
  WinuxCmd/coreutils; use WPM when utilities or command links are missing.
- Write generated edits into a temporary directory first. Do not use shell
  redirection to overwrite original source, config, or skill files. Redirect
  only into generated/temp files, inspect or diff the result, then replace with
  a deliberate `mv` or editor operation.
- In read-only or sandboxed validation, do not repair installs. Report the
  blocked surface, then use an explicit user-provided binary or repo bundle only
  as a clearly labeled fallback.
- Keep `winuxsh -c`, script files, and stdin execution quiet and deterministic:
  no banners, stable stdout/stderr, and exact exit-code propagation.
- Treat `~/.winshrc.toml` as the structured control plane and `~/.winshrc` as
  interactive REPL shell code. `~/.winshrc` must not be required for command
  mode, scripts, stdin execution, agent tests, or CI.
- Use newer docs and source code over old roadmap notes. Zsh-related surfaces
  are legacy migration/importer material unless the user explicitly asks about
  that migration path.
## Reference Map
- `references/runner.md`: `winuxsh -c`, Codex/pwsh quoting, Codex CLI/tool
  wrappers, stdin, scripts, positional args, active binary checks, and Windows
  path handling.
- `references/bash-sh-syntax.md`: Bash/sh syntax, pipelines, redirects,
  heredocs, jobs, arrays, functions, command discovery, and edit safety.
- `references/winuxcmd-discovery.md`: WinuxCmd PATH integration, WPM search,
  install/update/link rebuild workflows, activation diagnostics, utility
  provider collisions, and help.
- `references/config.md`: `~/.winshrc.toml`, `~/.winshrc`, prompt/completion,
  WinuxCmd overrides, test isolation, and two config surfaces.
- `references/plugins.md`: `winuxsh --self-update`, WPM package updates,
  `winuxsh plugin ...`, `oh-my-winuxsh` bundle updates, rollback, and review.
- `references/development.md`: repository ownership boundaries, command runner,
  target binary selection, Cargo test surfaces, and local upstream Bash gate.
## Avoid
- Do not present current Winuxsh work as zsh compatibility work. Keep zsh notes
  short and migration-only.
- Do not hardcode local machine paths, installed user names, per-user install
  directories, or one developer's PATH into open-source skill text.
- Do not hardcode a static utility inventory. Ask the active installation with
  `command -v`, command `--help`, `wpm list`, `wpm info`, and `wpm links list`.
- Do not fix Bash language behavior in Winuxsh host code when the bug belongs
  in rubash, and do not reintroduce WinuxCmd FFI/DLL routing.
