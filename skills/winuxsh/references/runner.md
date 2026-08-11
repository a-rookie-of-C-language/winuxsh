# Launching Winuxsh From External Hosts

Use this reference only when the current command runner is not already a
Winuxsh session, or when a task deliberately tests a fresh child shell. In a
native Winuxsh tool session, run commands directly and do not wrap ordinary
work in `winuxsh -c`.

## Boundary Rule

When a non-Winuxsh host such as PowerShell, cmd.exe, a GUI launcher, or another
agent host must start Winuxsh, use `winuxsh -c` as the process boundary:

```powershell
winuxsh -c 'printf "%s\n" "$PWD"'
winuxsh -c 'printf "%s\n" alpha beta | grep beta'
```

The host starts one Windows process. Winuxsh/rubash owns the program inside the
`-c` string, including pipes, redirects, variables, command substitution,
globs, and control flow. Do not wrap only part of a pipeline in Winuxsh and let
the host own the rest.

## Native Session Equivalent

Inside Winuxsh, use the same Bash program directly:

```bash
printf "%s\n" "$PWD"
printf "%s\n" alpha beta | grep beta
command -v winuxsh
command -v winuxcmd.exe || true
```

Nest `winuxsh -c` only for explicit child-shell testing:

```bash
winuxsh -c 'echo script-mode'
target/debug/winuxsh.exe -c 'echo checked-out-build'
```

## Quoting From PowerShell

If PowerShell or another host must launch Winuxsh, single-quote the `-c`
program so `$`, `$(...)`, `;`, and `|` reach Winuxsh. If quoting becomes hard
to audit, use a `.winux`/`.sh` script file or stdin instead of adding escapes.

```powershell
winuxsh -c 'x=ok; printf "%s\n" "$x"'
winuxsh C:/Users/me/project/scripts/check.winux alpha beta
```

PowerShell can mangle native executable arguments. If inline `-c` arrives
trimmed, split, or fails to parse only through the host, reproduce through a
script file before debugging Winuxsh.

## Path Rules

Prefer Windows-native paths:

```bash
cd C:/Users/me/project
test -f C:/Users/me/project/Cargo.toml
printf "%s\n" "C:\Users\me\project"
```

Use `/c/...` only for compatibility probes. Do not assume `/mnt/c`, MSYS2,
Git Bash, Cygwin, WSL, or `/usr/bin` layout.

## Permission-Looking Failures

When launching from external hosts, failures can come from the host wrapper,
sandboxing, Windows permissions, cwd resolution, or PATH provider differences.
If `cd`, writes, installs, link rebuilds, or file traversal fail with access
denied, read-only filesystem, sharing violation, or missing cwd symptoms,
classify it as likely permission/escalation before changing code.

Use the environment's approval/escalation mechanism for necessary privileged
actions. If escalation is unavailable, report the exact path/action and the
command that needs approval.
