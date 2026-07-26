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
- `rubash` PR #10 moved repeated loop body/condition AST construction out of
  per-iteration hot paths and was merged as
  `8026e3bfa81f694646f13786242d8d8ebca79ab4`.

## 2026-07-26 benchmark snapshot

These timings use the 80x80 microbenchmarks in
`C:\Users\caomengxuan\repo\tmp\rubash-perf` after pinning `winuxsh` to
`rubash` `8026e3bfa81f694646f13786242d8d8ebca79ab4`.

| Script | Bash ms | winuxsh release ms | Median ratio |
| --- | ---: | ---: | ---: |
| `loop.sh` | 201.6, 160.3, 160.1 | 1329.0, 677.1, 653.3 | 4.2x |
| `arith.sh` | 179.1, 172.7, 188.9 | 819.8, 854.3, 823.8 | 4.6x |
| `function-noop.sh` | 206.2, 203.8, 211.0 | 1118.5, 1083.1, 1123.2 | 5.4x |
| `function-args-arith.sh` | 279.4, 303.4, 303.4 | 1656.5, 1603.5, 1632.7 | 5.4x |

Direct `rubash` benchmarking before the `winuxsh` dependency bump showed the
loop-only case improving from roughly 635-996 ms to 532-536 ms, and the
function-plus-args-plus-arithmetic case improving from roughly 1655-1710 ms to
1498-1550 ms. That confirms the merged `rubash` optimization helps, but the
remaining gap is still large enough to keep issue #18 open for more profiling.

Next profiling targets should be `Executor::execute_ast`,
`expand_embedded_parameters`, shell function dispatch, and arithmetic evaluation
before changing `winuxsh` host code.

## Follow-up

- Add a checked-in ignored benchmark only after the release-mode baseline is
  captured on CI or a dedicated Windows runner.
- Keep the benchmark out of normal `cargo test`; it is timing-sensitive.
