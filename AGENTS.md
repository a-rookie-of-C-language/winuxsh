# Winuxsh Agent Rules

## Command Runner

- Do not use PowerShell or pwsh as the project command language.
- Run project commands through `winuxsh -c '<command>'`. If a host tool has to launch the process, the launched command should still be `winuxsh -c`.
- Ensure `winuxsh` resolves to the intended installed user binary, usually from `PATH` or a user tools directory outside this repository such as `~/tools`. Do not commit developer-local absolute runner paths.
- Do not let `PATH` resolve `winuxsh` to this repository's `target/release` or `target/debug` unless deliberately testing that exact build; those binaries can be stale or locked during builds.
- Use checked-out binaries only when deliberately testing that exact build, for example `target/debug/winuxsh.exe --version` after `cargo build`.
- Keep commands Windows-native. Use normal Windows paths (`C:/Users/<user>/...` or `C:\Users\<user>\...`) and do not introduce MSYS2, Git Bash, Cygwin, or WSL assumptions.
- If `ls`, `grep`, `tr`, or similar Unix commands are missing, fix the winuxcmd command-link setup rather than switching shells. A release-style bundle needs `winuxcmd.exe` plus generated command links in `PATH`.

## Product Direction

- Winuxsh is a Windows-native, non-isolated bash-compatible shell for humans and agents.
- Rubash is the shell language engine and is embedded as a library. Parser, executor, builtins, functions, redirects, pipelines, and job semantics belong upstream in `unixwin/rubash`.
- Keep rubash on the latest `unixwin/rubash` `master`. We have Unixwin organization access, so fix rubash upstream instead of carrying long-term host-side semantic workarounds in winuxsh.
- WinuxCmd stays integrated through PATH injection and command links. Do not reintroduce FFI/DLL command routing.
- Zsh compatibility is now a migration and maintenance layer, not the main product roadmap. Keep the scanner/importer safe, but do not expand toward a zsh interpreter or ZLE runtime.
- The future plugin system is Winuxsh-native WASM/WASI with a narrow host API. Process plugins can remain useful for debugging or compatibility, but they are not the destination.

## Development Rules

- Preserve quiet, deterministic non-interactive behavior for `winuxsh -c` and script execution: no banners, stable stdout/stderr, exact exit-code propagation.
- Keep interactive UX features in winuxsh/reedline unless they require shell semantics; shell semantics move to rubash.
- Keep compatibility tests honest: `tests/compat.rs` requires winuxcmd command links in `PATH`, not just a bare `winuxcmd.exe`.
- Keep one authoritative dependency lock at the repository root. Do not let `crates/winuxsh-runtime/Cargo.lock` drift from the binary build.
- When changing rubash, update the root lockfile and verify winuxsh through the root package.

## Verification

- Fast loop: `winuxsh -c 'cargo fmt --check -p winuxsh; cargo build --locked; cargo test --workspace --locked'`
- Runtime library: `winuxsh -c 'cargo test -p winuxsh-runtime --lib --locked'`
- Zsh/import maintenance: `winuxsh -c 'cargo test -p winuxsh-runtime --test zsh_compat --locked'`
- Host contract requiring winuxcmd command links: ensure command links are in `PATH`, then run `winuxsh -c 'cargo test --test host_contract --locked -- --ignored'`
- Compat suite: ensure winuxcmd command links are in `PATH`, then run `winuxsh -c 'cargo test --test compat --locked -- --ignored'`
- Local GNU Bash upstream gate: `winuxsh -c 'BASH_RUNNER="${BASH_RUNNER:-bash}"; "$BASH_RUNNER" scripts/run-bash-upstream-with-winuxsh.sh'` must report `86` total, `86` passed, `0` failed for the Winuxsh binary under test. Keep this local-only; do not add it to normal CI, and do not vendor Bash upstream tests into this repo. See `DOCS/bash-upstream-local.md`.
