---
name: winuxsh
description: >-
  Teach AI agents and users how to use Winuxsh as a Windows-native GNU
  Bash/sh-compatible shell from PowerShell, cmd, other host shells, or from
  inside Winuxsh itself. Use when running commands through winuxsh -c, writing
  Bash/sh scripts for Winuxsh, discovering available WinuxCmd executables and
  command help from the installed toolchain, or making small ~/.winshrc.toml
  configuration changes.
---

# Winuxsh

Use Winuxsh as the shell. Keep this file small; load references only when the
task needs them.

## Decide The Context

- If the current host is PowerShell, cmd, or another shell, run shell work with
  `winuxsh -c '...'` or `winuxsh script.sh`.
- If already inside Winuxsh, write normal Bash/sh commands directly. Do not
  nest `winuxsh -c` unless a fresh child shell is useful.
- If the user asks for shell syntax, write GNU Bash/sh syntax first, not
  PowerShell syntax.
- If the task needs Unix tools, discover WinuxCmd through WPM first when
  available, then fall back to executable-directory inspection.

## Load References

- For host-shell invocation, quoting, and "am I inside Winuxsh?" workflow, read
  `references/runner.md`.
- For Bash/sh syntax patterns Winuxsh understands, read
  `references/bash-sh-syntax.md`.
- For discovering WinuxCmd executables, command help, and alias collisions, read
  `references/winuxcmd-discovery.md`.
- For small user config edits only, read `references/config.md`.

## Defaults For Agents

- Prefer `winuxsh -c 'command'` over translating Bash requests to PowerShell.
- Use single quotes around the `-c` argument from PowerShell when the command
  contains `$`, `;`, pipes, redirects, or quotes that PowerShell might expand.
- Prefer script files or stdin for multiline shell programs instead of fragile
  host-shell escaping.
- Use WPM, `winuxcmd.exe <command> --help`, `man <command>`, and executable
  directory inspection to discover behavior. Do not paste a static command
  table into answers.
- Keep config guidance minimal unless the user explicitly asks for settings.

## Avoid

- Do not create a separate `winuxcmd` skill for Winuxsh usage.
- Do not make this about changing Winuxsh itself.
- Do not assume PowerShell aliases such as `ls`, `cat`, or `man` behave like
  WinuxCmd; run them through Winuxsh.
- Do not source arbitrary `.zshrc` or Oh My Zsh plugin code. Use Winuxsh's zsh
  report/import commands when migration is requested.
