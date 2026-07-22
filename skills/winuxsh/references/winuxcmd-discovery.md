# Discovering WinuxCmd Commands With Progressive Disclosure

Use this reference when the task needs Unix tools inside Winuxsh. Discover the
installed WinuxCmd surface instead of assuming a fixed command table. Prefer
WPM when available because it is WinuxCmd's own package/link manager and can
quickly expose the active executable set.

## Locate WinuxCmd

```bash
command -v winuxcmd.exe
cmd_dir=$(dirname "$(command -v winuxcmd.exe)")
printf "%s\n" "$cmd_dir"
```

If `winuxcmd.exe` is missing, Winuxsh may still run shell builtins, but Unix
tools such as `ls`, `grep`, `find`, `cp`, `mv`, and `rm` may not be available.
Ask the user to install or expose WinuxCmd rather than substituting PowerShell
aliases.

## Probe WPM First

Modern WinuxCmd builds include WPM as an internal command and often expose a
`wpm.exe` hardlink after activation.

```bash
winuxcmd.exe wpm version
winuxcmd.exe wpm links list --root "$cmd_dir"
winuxcmd.exe wpm index status --root "$cmd_dir"
winuxcmd.exe wpm list --root "$cmd_dir"
```

Interpretation:

- `wpm links list` discloses the command-link surface WinuxCmd can expose.
- `wpm index status` discloses the local/bundled package index and package
  count without fetching network data.
- `wpm list` discloses indexed packages such as `winuxcmd`, `jq`, `ncat`, `7zip`,
  `zstd`, or `yq` when present in the installed index.
- If `winuxcmd.exe wpm ...` reports `command not found: wpm`, the installed
  WinuxCmd is older; fall back to directory inspection.

Do not assume `wpm.exe` exists. Prefer `winuxcmd.exe wpm ...` because it works
before command links are rebuilt.

## Rebuild Links When Appropriate

If `wpm` exists but common commands such as `ls.exe` or `grep.exe` are missing,
rebuild links in the selected WinuxCmd root:

```bash
winuxcmd.exe wpm links rebuild --root "$cmd_dir" --force
```

Use this only for the WinuxCmd directory the user or bundle actually uses. Do
not rebuild links in an unrelated winget or development install.

## Fallback: List Available Executables

List command shims beside `winuxcmd.exe`:

```bash
cmd_dir=$(dirname "$(command -v winuxcmd.exe)")
ls "$cmd_dir" | sed -n 's/\.exe$//p' | sort
```

Also inspect WinuxCmd's own command index:

```bash
winuxcmd.exe --help
winuxcmd.exe help
```

Do not assume `winuxcmd.exe --list` exists.

## Get Command Help

Use one of these forms:

```bash
winuxcmd.exe grep --help
man grep
grep --help
```

When outside Winuxsh, wrap the lookup through Winuxsh:

```powershell
winuxsh -c 'winuxcmd.exe grep --help | head -40'
winuxsh -c 'man ls | head -60'
```

For WPM itself:

```bash
winuxsh -c 'winuxcmd.exe wpm | head -40'
winuxsh -c 'winuxcmd.exe wpm info winuxcmd --root "$(dirname "$(command -v winuxcmd.exe)")"'
```

## Avoid Host-Shell Alias Collisions

PowerShell aliases can shadow names such as `ls`, `cat`, and `man`. To inspect
WinuxCmd behavior, run inside Winuxsh or call explicit `.exe` names:

```powershell
winuxsh -c 'ls -la'
winuxsh -c 'man grep'
```

Inside Winuxsh, prefer normal Unix command names after verifying they resolve:

```bash
command -v ls
command -v grep
command -v find
```
