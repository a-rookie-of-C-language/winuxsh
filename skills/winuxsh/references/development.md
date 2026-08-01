# Developing Winuxsh

Use this reference before changing the winuxsh repository itself.

## Read Local Rules

Read `AGENTS.md` first. The live project rule is to run project commands
through `winuxsh -c '<command>'` and keep commands Windows-native. Ensure
`winuxsh` resolves to the intended installed user binary unless deliberately
testing a checked-out build.

## Component Ownership

- rubash owns shell language semantics: lexer, parser, AST, expansion,
  builtins, functions, redirects, pipelines, status propagation, jobs, and
  script execution.
- Winuxsh owns Windows-native host behavior: entry point, REPL, config, prompt,
  history, completion frontend, migration importers, plugin host, Windows
  Terminal integration, self-update, Ctrl+C, PATH injection, deterministic
  non-interactive execution, and the one-shot REPL command surface.
- WinuxCmd owns external Unix-style utilities exposed as Windows executables
  through PATH and command links.

Fix behavior in the owning layer. Do not add Winuxsh host-side parser/executor
workarounds for rubash bugs, and do not reintroduce WinuxCmd FFI/DLL routing.

## Command Runner

Fast loop:

```bash
winuxsh -c 'cargo fmt --check -p winuxsh; cargo build --locked; cargo test --workspace --locked'
```

Focused loops:

```bash
winuxsh -c 'cargo test -p winuxsh-runtime --lib --locked'
winuxsh -c 'cargo test --test completion_probe --locked'
winuxsh -c 'cargo test --test repl_command --locked'
winuxsh -c 'cargo test --test plugin_inventory --locked'
```

Migration/importer maintenance only:

```bash
winuxsh -c 'cargo test -p winuxsh-runtime --test zsh_compat --locked'
```

Host-contract and compatibility tests require WinuxCmd command links in the
Windows process `PATH`. When prepending a local WinuxCmd build for these Cargo
tests, use the Windows separator (`;`) so both `std::process::Command` and
rubash child commands see the links:

```bash
winuxsh -c 'PATH="C:/path/to/WinuxCmd/build-vs-release;$PATH" WINUXCMD_PATH="C:/path/to/WinuxCmd/build-vs-release/winuxcmd.exe" cargo test --test host_contract --locked -- --ignored'
winuxsh -c 'PATH="C:/path/to/WinuxCmd/build-vs-release;$PATH" WINUXCMD_PATH="C:/path/to/WinuxCmd/build-vs-release/winuxcmd.exe" cargo test --test compat --locked -- --ignored'
```

If `ls`, `grep`, `tr`, or similar commands are missing, repair the WinuxCmd
command-link setup instead of switching to pwsh or another shell.

## One-Shot REPL Command Development

`-C` / `--repl-command` is the non-interactive REPL command path. It should load
REPL startup shell code, run lifecycle hooks, execute one REPL-style line, exit
with the command status, and stay banner-free. It is meant to solve agent and
host-shell pain where a one-shot command needs the same REPL state as an
interactive user without opening a real interactive session.

Keep `-c` separate: it is the quiet script/CI path and must not require
`~/.winshrc` or REPL lifecycle hooks. When this area changes, test both sides:

```bash
winuxsh -c 'cargo test --test repl_command --locked'
target/debug/winuxsh.exe --help
target/debug/winuxsh.exe -C 'echo one-shot-repl'
target/debug/winuxsh.exe -c 'echo script-mode'
```

Do not document `-C` as generally available until the target binary's
`winuxsh --help` shows it.

## Testing A Just-Built Binary

Use checked-out binaries only when deliberately testing that exact build:

```bash
winuxsh -c 'cargo build --locked; target/debug/winuxsh.exe --version; target/debug/winuxsh.exe -c "printf %s\\n ok"'
```

Otherwise, avoid adding `target/debug` or `target/release` ahead of the
installed user binary in PATH. Stale or locked binaries can hide real behavior.

## Local Bash Upstream Gate

The upstream Bash local gate is intentionally external to normal CI:

```bash
winuxsh -c 'BASH_RUNNER="${BASH_RUNNER:-bash}"; "$BASH_RUNNER" scripts/run-bash-upstream-with-winuxsh.sh'
```

Expected pass condition is `86` total, `86` passed, `0` failed for the Winuxsh
binary under test. If `bash` has another name, set `BASH_RUNNER`. If upstream
Bash tests live elsewhere, set `BASH_UPSTREAM_DIR`. Do not vendor upstream Bash
tests into this repo.

## Regression Surfaces

Choose tests by behavior:

- CLI/help/version/plugin parsing: root binary tests in `tests/`.
- One-shot REPL command behavior: `tests/repl_command.rs` for `-C` loading
  `~/.winshrc`, running lifecycle hooks, staying banner-free, and preserving
  `-c` script semantics.
- Runtime config, prompt, completion, shell host integration:
  `winuxsh-runtime` library tests.
- Completion UX: `--completion-probe` and `tests/completion_probe.rs`.
- Plugin inventory/bundles/process/WASM: `tests/plugin_inventory.rs`.
- Bash semantics: rubash upstream first, then Winuxsh integration tests.
- Legacy migration importer: `zsh_compat` and binary-level migration tests.

Preserve quiet `winuxsh -c`, `winuxsh -C`, and script execution. User-facing
REPL polish belongs in interactive paths unless the change explicitly targets
the one-shot REPL command surface.
