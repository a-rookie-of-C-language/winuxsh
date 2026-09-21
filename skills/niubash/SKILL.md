---
name: niubash
description: Run Windows tasks in Niubash, the GNU Bash-compatible Windows-native shell. Use when the active shell is Niubash (the `niu` prompt), when editing ~/.niubashrc or oh-my-niu plugins, when installing Unix commands with wpm, or for any Windows task where bash syntax should run natively — no PowerShell, WSL, Git Bash, or MSYS involved, and no path-conversion environment variables required.
---

# Niubash

Niubash is bash implemented natively in Rust for Windows. GNU Bash's own
upstream test suite runs as its gate — 86/86 selected tests, 57/83 full
suites byte-identical to GNU output. There is no emulation layer and **no
path-conversion machinery**, which is why — unlike MSYS-family environments —
you never need `MSYS_NO_PATHCONV`, `MSYS2_ARG_CONV_EXCL`, or any other
bridging variable. There is nothing to disable, because there is no
converter.

## Core rules

- The current session already **is** Niubash. Run commands directly; do not
  nest `niu -c`, and do not route work through `pwsh`, `powershell`,
  `cmd /c`, `wsl`, or `bash` wrappers.
- Write bash syntax normally: functions, arrays, `$(...)`, pipes, heredocs,
  globs. For advanced bash-isms, test in-session before relying on them
  (see parity notes below).
- Paths: native `C:/Users/me` (or `C:\Users\me`) is always safe and is what
  Windows executables should receive. `/c/...`, `/mnt/c/...`, `/usr/bin`,
  `/etc/...` are understood input dialects. `/tmp` is the **real Windows
  temp directory**, never the install tree. Full contract:
  `references/paths.md`.
- Discover, don't assume. Before using a tool: `command -v <tool>`. For the
  installation as a whole: `niu doctor`. For the command runtime:
  `winuxcmd --version`. For installed packages: `wpm installed`.
- A Unix command is missing? Install it with `wpm install <name>` — see
  `references/wpm.md` for search, links repair, index updates, and repair
  flows.

## Environment discovery

```bash
niu doctor                 # one-shot health check: core, links, rc, bundle
winuxcmd --version         # WinuxCmd command-link runtime version
wpm installed              # packages present in this installation
command -v rg              # is a specific tool resolvable?
```

Mental model: **wpm is the package manager** (downloads GNU/POSIX command
packages like `rg`, `fd`, `bat`, `jq`), **WinuxCmd is the command runtime**
that owns the real `usr/bin` tree and command links, and
`niu --self-update` updates the shell itself — three separate update paths.

## Bash parity — test, then trust

- The measured floor: 86/86 gate, 57/83 upstream suites byte-identical
  (ledger: rubash `docs/COMPATIBILITY-STATUS.md`). Strong, but not a
  guarantee that every bash-ism works.
- Before relying on `fc`, `coproc`, exotic redirects, `mapfile` edge cases,
  `compgen`/`complete`, or deep parameter expansion, test the exact form
  **in the current session** (not via `niu -c`, which adds a quoting layer).
- On divergence: fall back to a simpler POSIX form, then report the gap
  upstream to `unixwin/rubash`. Do not carry host-side workarounds.

## Configuration

- `~/.niubashrc` is the single interactive entry point: `NIU_*` exports,
  `NIU_PLUGINS=(prompt-core git)` arrays, aliases, functions, PATH prepends.
  Structured manifests are not user startup config.
- PATH additions: prefer idempotent rc-local prepends over editing the
  Windows user PATH:

  ```bash
  [ -d "C:/tools/bin" ] && case ":$PATH:" in *":C:/tools/bin:"*) ;; *) PATH="C:/tools/bin;$PATH";; esac
  ```

- Never blind-source an rc you did not write; check for self-appending
  blocks (`>> ~/.niubashrc`), install commands, or network calls first.
- oh-my-niu bundles load from the rc via `oh-my-niu.niu` (legacy
  `oh-my-niu.winux` still honored). User plugins live in
  `~/.niubash/custom`; do not edit files inside the installed program
  directory. Theme names, the pack catalog, and rc templates:
  `references/prompt-plugins.md`.
- `niu setup` re-runs the interactive wizard (theme, prompt, tools). The
  wizard follows `NIU_LANG` / the Windows UI language (zh supported).
- `niu font` installs a Nerd Font for icon themes; `niu doctor` verifies the
  whole installation.

## Validation

- Verify in-session: source a temporary file or run the snippet directly
  before editing rc files; then confirm with `command -v`, `alias`, or
  `test -f C:/path`.
- Validate installs with direct calls: `rg --version`, `wpm links list`.
- Reserve `niu -c "..."` for when an external host must launch Niubash
  non-interactively: it is quiet, deterministic, and propagates exit codes
  exactly — that is its contract.

## Session context

- If the host tool is already executing inside Niubash, everything above
  applies directly.
- If the host runs another shell, invoke Niubash explicitly (`niu -c "..."`)
  for bash-syntax work, and keep native `C:/...` paths for anything passed
  to Windows executables.
