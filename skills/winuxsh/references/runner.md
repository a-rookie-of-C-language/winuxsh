# Running Winuxsh

Use this reference when Codex or another host shell needs to launch Winuxsh,
verify which binary is active, pass Bash safely through pwsh, or handle Windows
paths without inventing a Unix filesystem.

## Mental Model

Use `winuxsh -c` as the boundary: the host starts one Windows process, and
Winuxsh/rubash owns the shell program inside `-c`. Pipes, redirects, variable
expansion, command substitution, globs, and control flow should be Bash syntax
inside the `-c` string, not pwsh syntax around it.

There is no `/usr/bin` target layer. Utilities resolve through the Windows
process PATH, usually with WinuxCmd command links placed ahead of other
providers.

## Verify The Active Binary

From Codex/pwsh or another Windows host:

```powershell
where.exe winuxsh
winuxsh --version
winuxsh -c 'command -v winuxsh; command -v winuxcmd.exe || true; command -v wpm || true'
```

Inside Winuxsh:

```bash
command -v winuxsh
winuxsh --version
command -v winuxcmd.exe || true
command -v wpm || true
```

Prefer the installed user binary from PATH or the user's selected tools
directory. Do not accidentally test `target/debug/winuxsh.exe` or
`target/release/winuxsh.exe` unless the task deliberately targets that build.

If no active PATH binary is visible in a read-only sandbox, do not mutate PATH
or rebuild links. Report that the active install is unavailable. If the user
provided an explicit binary or a repo bundle is part of the task, you may probe
that binary as a labeled fallback and keep its results separate from the active
install.

## Codex/pwsh Invocation

In pwsh, use single quotes around the `-c` program. This was verified with
Bash variables and pipelines: single quotes let `$x`, `$(...)`, `;`, and `|`
reach Winuxsh; double quotes let pwsh expand `$x` before Winuxsh sees it.

```powershell
winuxsh -c 'x=ok; printf "%s\n" "$x"'
winuxsh -c 'printf "%s\n" alpha beta | grep beta'
winuxsh -c 'if [ -f Cargo.toml ]; then echo rust; else echo no-manifest; fi'
winuxsh -c 'false; printf "status=%s\n" "$?"'
```

Do not wrap only part of a pipeline in Winuxsh:

```powershell
# Wrong for Winuxsh validation: pwsh owns the pipe.
winuxsh -c 'printf "%s\n" alpha beta' | Select-String beta

# Right: Bash owns the pipe.
winuxsh -c 'printf "%s\n" alpha beta | grep beta'
```

Use pwsh double quotes only for trivial commands or when host interpolation is
intentional and audited. If a program needs nested single quotes, move the
logic to a script file or feed a script on stdin rather than piling on escaping.

## Command Mode Vs One-Shot REPL

Use `-c` for deterministic script/CI/agent command mode. It executes through
the script path and must not require `~/.winuxshrc`, legacy `~/.winshrc`,
prompt state, or interactive REPL hooks:

```powershell
winuxsh -c 'printf "%s\n" "$PWD"'
```

Use `-C` / `--repl-command` only when the task specifically needs one
non-interactive REPL-style command: load REPL startup shell code, run lifecycle
hooks, use REPL line execution behavior, then exit without an interactive
session or banner. This is a newer surface under active development, so verify
support on the exact target binary first:

```powershell
winuxsh --help
winuxsh -C 'echo rc:$WINUXSH_REPL_COMMAND_RC'
winuxsh --repl-command 'echo one-shot-repl'
```

Keep the distinction explicit in reports: `-c` validates script semantics;
`-C` validates REPL startup/hook semantics. Do not silently replace one with
the other.

## Codex CLI And Tool Wrappers

Codex CLI and app tools may show a pwsh command wrapper because the host shell
is pwsh. That is acceptable only as the process launcher. For Winuxsh work, the
actual command inside the wrapper should still be `winuxsh -c '...'` or an
explicit `winuxsh.exe` path plus `-c`.

```powershell
codex --ask-for-approval never exec --ephemeral -s read-only -C C:/Users/me/project "Use winuxsh -c for all project commands; do not edit files."
```

The Codex CLI flag `-C C:/Users/me/project` in the example sets the Codex
working directory; it is unrelated to Winuxsh `-C` / `--repl-command`. When
invoking the Winuxsh one-shot REPL command through Codex, put `winuxsh -C ...`
inside the prompt or launched command explicitly.

When reviewing subagent or CLI output, flag probes such as `Get-Content`,
`Select-String`, or pwsh-owned pipes as host-shell work. They may be acceptable
for generic read-only file inspection, but they do not validate Winuxsh command
behavior.

## Script Files, Stdin, And Args

For multiline logic, prefer a script file:

```powershell
winuxsh C:/Users/me/project/scripts/check.sh alpha beta
```

Pass positional args to `-c` by providing a placeholder script name, then args:

```powershell
winuxsh -c 'printf "%s\n" "$1"' _ alpha
```

When Winuxsh starts non-interactively with no script argument, it can read a
script from stdin. Use stdin for generated probes. For generated file changes,
write outputs under a temporary directory and only then move the reviewed
result into place; never point redirection at an original file. Use a
checked-in or temp script file for repeatable work.

## Windows Path Rules

Prefer native Windows path input:

```powershell
winuxsh -c 'cd C:/Users/me/project; pwd'
winuxsh -c 'test -f C:/Users/me/project/Cargo.toml && echo ok'
winuxsh -c 'printf "%s\n" "C:\Users\me\project"'
```

Guidance:

- Use `C:/...` in scripts and examples because it survives Bash and host-shell
  quoting with the fewest escapes.
- Use `C:\...` when the task specifically tests native backslash input. Quote
  it carefully across host boundaries.
- Use `/c/...` only as compatibility input or when matching output from a
  specific release under test.
- Do not use `/mnt/c/...`, MSYS2 roots, Git Bash roots, Cygwin roots, or
  `/usr/bin` assumptions unless the task is explicitly about rejecting or
  comparing those environments.

Some target binaries may still display a `/c/...`-style normalized `pwd` while
accepting native drive input. Treat that as target-version behavior to verify,
not as permission to model Winuxsh as a Unix-rooted environment.

## Basic Smoke Tests

```powershell
winuxsh --version
winuxsh -c 'printf "winuxsh ok\n"; pwd; command -v winuxsh; command -v winuxcmd.exe || true'
winuxsh -c 'printf "%s\n" alpha beta | grep beta'
winuxsh -c 'x=(one two); printf "%s\n" "${x[1]}"'
```

If `winuxsh` is not found, ask the user to expose the chosen `winuxsh.exe` on
Windows PATH. Do not invent aliases such as `winux` unless the user asks.
