# Bash/sh Syntax In Winuxsh

Use this reference when writing commands or scripts for Winuxsh. Use GNU
Bash-compatible syntax and verify uncertain behavior against the active binary.
Rubash owns parsing, expansion, builtins, redirects, pipelines, jobs, and script
execution.

## One-Liners

```bash
pwd
ls -la
grep -n "TODO" README.md
find . -name "*.rs"
test -f Cargo.toml && echo "has manifest"
false && echo bad || echo fallback
```

Prefer `printf` for deterministic output in tests. Use `echo` for simple
interactive examples.

## Variables And Substitution

```bash
name=winuxsh
printf "%s\n" "$name"
printf "%s\n" "$(git rev-parse --short HEAD)"
printf "%s\n" "$PWD"
```

Quote variable expansions unless word splitting is intentional.

## Windows Paths

Use native Windows paths rather than a virtual Unix tree:

```bash
cd C:/Users/me/project
test -f C:/Users/me/project/Cargo.toml
printf "%s\n" "C:\Users\me\project"
```

Use `C:/...` for scripts and examples. Use `C:\...` for intentional backslash
coverage. Use `/c/...` only for compatibility checks. Do not assume `/usr/bin`,
`/mnt/c`, MSYS2, Cygwin, Git Bash, or WSL layout.

## Conditionals, Loops, And Case

```bash
if [ -f Cargo.toml ]; then
  echo "Rust project"
else
  echo "No manifest"
fi

for item in one two three; do
  printf "%s\n" "$item"
done

case "$1" in
  build) cargo build --locked ;;
  test) cargo test --workspace --locked ;;
  *) printf "usage: %s {build|test}\n" "$0" >&2; exit 2 ;;
esac
```

## Functions And Arrays

```bash
greet() {
  printf "hello %s\n" "$1"
}

items=(alpha beta gamma)
greet "${items[1]}"
```

## Pipes, Redirects, And Heredocs

```bash
printf "%s\n" alpha beta | grep beta
cat <<'EOF' > .tmp/generated-example.txt
literal $text is preserved here
EOF
```

Redirects are part of the Winuxsh experience, but for edits write into a
temporary directory first. Avoid `> original-file` for source, config, or skill
edits. Redirect to generated/temp files, inspect or diff the result, then move
or install it deliberately.

## Jobs And Status

```bash
sleep 1 &
pid=$!
jobs
wait "$pid"
printf "status=%s\n" "$?"
```

Job control can be host- and version-sensitive on Windows. Verify background
jobs, `jobs`, `wait`, Ctrl+C, and signal behavior on the exact target binary
before promising exact output.

## Builtins And External Commands

Rubash supplies shell builtins and shell semantics: `cd`, `source`, `export`,
`alias`, `test`, `printf`, functions, variables, redirects, pipelines, and exit
status. External commands such as `ls`, `cat`, `grep`, `find`, `cp`, `mv`,
`rm`, `jq`, `7zip`, and `yq` resolve through PATH, commonly from WinuxCmd
command links. Discover the provider before relying on flags.

Do not use host PowerShell, Python, Node, or awk to fake Bash behavior or
routine file-edit glue. If a utility is missing, use WPM or verify the intended
provider.

## Non-Bash Boundary

Use Bash/sh syntax. Do not depend on other-shell-only arrays, `autoload`,
`compdef`, `zstyle`, editor widgets, or startup execution. Legacy shell files
are migration inputs only when the user explicitly asks for that importer.
