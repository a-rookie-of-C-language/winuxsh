# Performance investigation: pure-sh nested loops

This note tracks issue #18: pure shell scripts with nested loops and heavy
function calls should stay predictable under `winuxsh -c` and script-file
execution.

## Scope

- Measure `winuxsh` in release mode, not debug mode.
- Compare against the same script under Git Bash or another local bash.
- Keep the benchmark script pure shell: arithmetic expansion, function calls,
  variable assignment, and loop control only.
- Record stdout, stderr, wall time, and exit status.

## Suggested harness

```powershell
$script = @'
inner() {
  x=$1
  y=$2
  printf ''%s\n'' $((x + y)) > /dev/null
}

i=0
while [ "$i" -lt 200 ]; do
  j=0
  while [ "$j" -lt 200 ]; do
    inner "$i" "$j"
    j=$((j + 1))
  done
  i=$((i + 1))
done
'@

Set-Content -NoNewline .tmp/perf-nested-loops.sh $script
cargo build --release
Measure-Command { target/release/winuxsh.exe .tmp/perf-nested-loops.sh }
Measure-Command { bash .tmp/perf-nested-loops.sh }
```

## Current findings

- The hottest expected paths are rubash parse/execution loops, arithmetic
  expansion, and per-command environment synchronization.
- Host-facing fixes in this batch avoid extra work on command startup: stdin
  inheritance and script positional parameters are simple executor state writes.
- If release-mode timings remain far from bash, profile `Executor::execute_ast`,
  `expand_embedded_parameters`, and arithmetic evaluation before changing
  winuxsh host code.

## Follow-up

- Add a checked-in ignored benchmark only after the release-mode baseline is
  captured on CI or a dedicated Windows runner.
- Keep the benchmark out of normal `cargo test`; it is timing-sensitive.
