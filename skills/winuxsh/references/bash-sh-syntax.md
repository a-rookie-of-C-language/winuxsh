# Bash And sh Syntax In Winuxsh

Use this reference when writing commands or scripts for Winuxsh. Prefer GNU
Bash/sh style syntax and only fall back to PowerShell when the user explicitly
asks for PowerShell.

## One-Liners

```bash
pwd
ls -la
grep -n "TODO" README.md
find . -name "*.rs"
test -f Cargo.toml && echo "has manifest"
false && echo bad || echo fallback
```

## Variables And Substitution

```bash
name=winuxsh
echo "$name"
echo "$(git rev-parse --short HEAD)"
printf "%s\n" "$PWD"
```

Use double quotes around variable expansions unless word splitting is intended.

## Conditionals And Tests

```bash
if [ -f Cargo.toml ]; then
  echo "Rust project"
else
  echo "No manifest"
fi
```

## Loops

```bash
for item in one two three; do
  echo "$item"
done

while read line; do
  echo "line=$line"
done < input.txt
```

## Functions

```bash
greet() {
  printf "hello %s\n" "$1"
}

greet winuxsh
```

## Pipes, Redirects, And Heredocs

```bash
printf "%s\n" alpha beta | grep a
grep -n TODO README.md > todo.txt
cat <<'EOF' > script-output.txt
literal $text is preserved here
EOF
```

## Script Execution

Use `.sh` scripts for larger programs:

```powershell
winuxsh script.sh
```

For agents invoking from PowerShell, keep Bash syntax inside the `-c` string:

```powershell
winuxsh -c 'for i in 1 2 3; do echo "$i"; done'
```

Do not rewrite these examples into PowerShell syntax unless requested.
