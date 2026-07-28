# GNU Bash upstream local gate

Winuxsh keeps the GNU Bash upstream compatibility run as a local development
gate. Do not add the upstream Bash test tree to this repository, and do not run
the full gate in CI by default; it is intentionally too slow for the normal
Windows CI loop.

## Expected layout

Keep a sibling rubash checkout with its existing Bash upstream fixture:

```text
C:/Users/caomengxuan/repo/winuxsh
C:/Users/caomengxuan/repo/rubash/third_party/bash/tests
```

The runner can be pointed at another Bash upstream checkout with
`BASH_UPSTREAM_DIR`, but it must stay external to the winuxsh repository.

## Run command

Run through the installed Winuxsh command runner:

```sh
C:/Users/caomengxuan/tools/winuxsh.exe -c '"C:/Program Files/Git/bin/bash.exe" scripts/run-bash-upstream-with-winuxsh.sh'
```

The gate passes only when it reports:

```text
Total: 86
Passed: 86
Failed: 0
```

Results are written under:

```text
C:/Users/caomengxuan/repo/winuxsh/target/bash-upstream-tests
```

## Performance guardrails

The runner is test infrastructure and must not move into normal startup, `-c`,
script-file, or REPL hot paths. When touching Winuxsh host execution, follow the
Rubash performance process from the local perf worktree:

```text
C:/Users/caomengxuan/repo/wt-rubash-baseline/docs/performance-debugging-process.md
```

In practice:

- keep shell semantics in rubash and avoid host-side interpreter rewrites;
- avoid eager process environment synchronization in hot command loops;
- avoid rebuilding static command shape or materializing Bash arrays unless a
  script observes them;
- compare behavioral fixes against focused tests first, broad tests second, and
  the local upstream gate when Bash compatibility is affected.
