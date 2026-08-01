# Plugin Externalization Readiness
This document is the gate before moving official `builtin` packs into
code-bearing runtimes. It records what can be externalized now, what must stay
host-owned, and which host APIs are missing.
Do not add a new runtime kind or manifest field from this document alone. Use
this matrix to decide whether a pack belongs in `source`, `process`, `wasm`,
`builtin`, or a future asset-only representation.
## Current Host Facts
- WASM is command-only today. It can run `winuxsh_plugin_main() -> i32`, write
  deterministic stdout/stderr, read command args, read cwd with `cwd:read`, and
  read specific environment values with `env:read:<NAME>`.
- WASM does not yet support provider output, completions, prompt segments,
  lifecycle hooks, files, process access, env mutation, cwd mutation, history
  reads, or shell-command effects.
- Source plugins are reviewed, bundle-local `.winux` startup scripts. They can
  mutate the current interactive shell by design, require `shell:source`, load
  for the REPL and `-C`, and do not run for ordinary `winuxsh -c`, script
  files, or stdin execution.
- Process plugins can expose commands, lifecycle hooks, and the command-not-found provider binding. Hook stdout is
  not a structured effect protocol for mutating shell state.
- Static assets do not need WASM. Aliases, completion tables, prompt presets,
  keybinding metadata, and themes should be separated from "builtin behavior"
  before moving behavior into WASM or process runtimes.
- `plugin list`, `plugin info`, `plugin search`, `plugin review`, and
  `plugin doctor` expose derived `execution_model`, `externalization_class`,
  and readiness profile values. These are review surfaces, not manifest schema
  fields.
## Classification Terms
| Classification | Meaning |
| --- | --- |
| Declarative asset | Bundle-owned TOML or static files; no plugin code should run. |
| Mixed declarative/native | Static bundle assets plus host-owned native behavior. |
| Shell source | Bundle-local `.winux` startup code that intentionally mutates the current interactive shell. |
| Pure provider candidate | Mostly input -> output; good future WASM provider target. |
| External-tool adapter | Wraps an existing native command; process runtime may fit. |
| Shell-effect candidate | Mutates cwd/env/history/cache or executes suggested commands; wait for an effect protocol. |
| Fixture | Test/demo pack that proves host runtime behavior, not a normal user feature. |
## Readiness Matrix
| Pack | Current runtime | Classification | Target runtime / execution model | Missing host API or decision | Shell-mutating | Fallback needed |
| --- | --- | --- | --- | --- | --- | --- |
| `git` | `source` | Shell source plus mixed declarative/native | `.winux` startup helpers plus alias/completion assets; native prompt segment until prompt provider ABI | Prompt segment provider ABI for dynamic git status | Yes, by trusted startup source | Yes, compiled prompt segment fallback |
| `docker` | `source` | Shell source plus declarative asset | `.winux` helper functions plus alias/completion assets | None for current first-party source helper scope | Yes, by trusted startup source | Minimal, existing compiled aliases/completions |
| `kubectl` | `source` | Shell source plus declarative asset | `.winux` helper functions plus alias/completion assets | None for current first-party source helper scope | Yes, by trusted startup source | Minimal |
| `npm` | `source` | Shell source plus mixed declarative/native | `.winux` helper functions plus assets; native/dynamic completion until completion provider ABI | Completion/provider ABI if dynamic npm completion remains host-owned | Yes, by trusted startup source | Yes |
| `zoxide` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after cwd effects are explicit | `shell:cwd:write`, lifecycle context, rollback/failure behavior | Yes | Yes |
| `direnv` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after env writes are structured | `env:write`, scoped process adapter policy, lifecycle context, rollback/failure behavior | Yes | Yes |
| `dotenv` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after fs/env effects are structured | Scoped `fs:read`, `env:write`, lifecycle context, rollback/failure behavior | Yes | Yes |
| `fzf` | `builtin` | External-tool adapter plus shell effect | Native builtin now; future process/effect only after interactive policy | Interactive process policy, `shell:cwd:write`, rollback/failure behavior | Yes | Yes |
| `command-not-found` | `builtin` | Pure provider candidate | Process provider binding exists; official pack can stay builtin until migration; future WASM provider must use separate ABI | Bundle migration decision plus future WASM provider entrypoint | No | Yes |
| `last-working-dir` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after cache/cwd protocol | Scoped cache read/write, startup/chpwd effect protocol, `shell:cwd:write` | Yes | Yes |
| `thefuck` | `builtin` | External-tool adapter plus shell effect | Native/process adapter plus future effect protocol; hold now | `history:read`, suggested command review/execute protocol, process adapter policy | Yes | Yes |
| `keybindings` | `builtin` | Declarative asset | Asset-only/declarative; schema marker TBD | Schema decision: asset-only marker; reedline actions stay native | No | Minimal |
| `prompts` | `builtin` | Mixed declarative/native | Declarative prompt presets plus native segments until prompt provider ABI | Prompt segment provider ABI for dynamic values | No | Yes |
| `themes` | `builtin` | Declarative asset | Asset-only/declarative; schema marker TBD | Schema decision: asset-only marker; theme renderer stays native | No | Minimal |
| `process-echo` | `process` | Fixture | Process command fixture | None for current fixture scope | No | No |
| `process-hook` | `process` | Fixture | Process hook fixture; not a generic effect runtime | Structured hook effects before using as normal shell-mutating pack | No in fixture | No |
| `wasm-hello` | `wasm` | Fixture | WASM command fixture; provider/effect ABI is separate | Provider/effect ABI is separate future work | No | No |
## Immediate Decision Gates
1. Use `source` when the plugin's value is shell startup behavior and the
   `shell:source` trust tradeoff is acceptable.
2. Separate declarative assets from host-owned behavior in naming and docs
   before changing manifest schema.
3. Choose the asset-only representation only after checking bundle schema,
   index validation, compiled fallback inventory, and compatibility docs.
4. Use the implemented process-provider binding around
   [command-not-found](plugin-command-not-found-provider-abi.md):
   verify missing-command input, host context, deterministic suggestion output, and fallback before migrating the official pack.
5. Keep untrusted or third-party shell-mutating packs native/source-disabled
   until effects are explicit, permissioned, reversible when possible, and
   tested against failure/timeout behavior.
