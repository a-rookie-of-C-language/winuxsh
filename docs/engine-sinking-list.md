# Engine Sinking List (host-layer findings, 2026-09-22)

Companion to rubash `docs/niu-product-baseline-20260922.md` (P0-P5 product-layer
divergence elimination). Per the two-layer testing discipline (governance doc
3.7), these need semantic changes in the rubash engine and must NOT be hacked
around in the niu host layer. Baselines: WSL GNU Bash 5.3.0 via
`scripts/true-baseline.sh`; engine rubash @3aa37b3d; niu on branch
`fix/niu-product-divergences`.

## S1. Publish the script driver (histexp, script `set -H` expansion)

`rubash::script_driver` (`run_script_with_history`, `script_uses_history`,
`script_uses_aliases`, `run_source`) exists and is used by rubash's own
`main.rs:774` for scripts that enable history, but the module is not public at
3aa37b3d, so embedded hosts cannot route scripts through it. rubash's local
tree already carries the fix (module made `pub`, +66 lines in
`script_driver.rs`); pushing that lets niu replace its plain whole-script
`execute_script` path for history/alias scripts. Impact once sunk: histexp
175 -> engine-level (~74 at 3aa37b3d, ~1 with the local tree).

## S2. Engine DEBUG/RETURN trap at main-script EOF

After the last command of a script completes, niu prints one extra DEBUG/RETURN
trap line ("debug lineno: 1 main") where GNU and rubash.exe do not. Root cause:
rubash.exe executes scripts through its line-grouped driver, while niu uses the
whole-script `execute_script` path, which fires the traps once at EOF. Sunk
either by S1 (hosts inherit the same driver) or by an engine fix for whole-AST
execution. Suites: dbg-support +1 line, dbg-support2 +1 line (niu vs engine).

## S3. Engine emulated `cp` mishandles `src/.` into an existing directory

`cp -R src/. dst/` (and the relative-path form) silently skips the copy when
`dst` already exists as a directory; GNU copies the contents into it.
executor/external_file_builtins.rs `external_cp`. Registered while updating
`tests/host_contract.rs` to the P1 engine-delegation contract.

## S4. `history 10 42` must be "too many arguments" (rc 2)

With `set -o history`, GNU rejects two numeric arguments (rc 2). rubash
@3aa37b3d prints a history listing (rc 0); niu prints the usage line but still
rc 0. errors.tests errors10.sub observes this ("after history: $?").

## S5. `!!`/history expansion inside `-c` / whole-script execution

`bash -c` with `set -H` performs history expansion per GNU bashhist.c;
neither rubash nor niu expands `!!` in `-c`. Same driver dependency as S1.

## Environment-bound observations (not shell bugs)

- `errors.tests` baseline depends on the invoking shell's exported OLDPWD:
  GNU's `cd -` (errors.tests:227) changes the process cwd, after which
  `./errors10-12.sub` resolve against the wrong directory. Whether the GNU
  side therefore truncates depends on where `wsl bash` was started, which is
  why errors has measured 3, 115, and 1367 across runs. Run the harness from
  a fixed directory (repo root) for comparable numbers.
