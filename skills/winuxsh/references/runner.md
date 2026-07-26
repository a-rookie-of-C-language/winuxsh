# Running Winuxsh From Other Shells

Use this reference when an agent is currently in PowerShell, cmd, Git Bash, or
another shell and needs to run Winuxsh correctly.

## From PowerShell

Prefer single quotes around the `-c` program so PowerShell does not expand `$`,
`$(...)`, `;`, pipes, or redirections before Winuxsh sees them:

```powershell
winuxsh -c 'pwd; ls -la; echo "$PWD"'
winuxsh -c 'if [ -f Cargo.toml ]; then echo rust; fi'
winuxsh -c 'false && echo bad || echo fallback'
```

Use double quotes only for simple commands or when PowerShell interpolation is
intentional.

## From cmd.exe

Use `winuxsh -c "..."` for simple one-liners. For complex quoting, prefer a
script file:

```cmd
winuxsh -c "pwd; ls -la"
winuxsh script.sh
```

## From Inside Winuxsh

If already in Winuxsh, run Bash/sh directly:

```bash
pwd
ls -la
if [ -f Cargo.toml ]; then echo rust; fi
```

Use nested `winuxsh -c` only when a separate shell process is useful, such as
checking non-interactive behavior or isolating a command.

## Scripts And Stdin

For multiline logic, prefer a script over host-shell escaping:

```bash
winuxsh script.sh
```

Winuxsh also reads a script from stdin when launched non-interactively without
arguments, but `winuxsh script.sh` is clearer for repeatable work.

## Basic Verification

```powershell
winuxsh -c 'printf "winuxsh ok\n"; command -v winuxsh; command -v winuxcmd.exe'
winuxsh --version
```

If `winuxsh` is not found, add the directory containing `winuxsh.exe` to PATH.
Do not invent aliases such as `winux` unless the user explicitly asks.
