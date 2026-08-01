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
  keybinding metadata, and themes are bundle-owned assets consumed by the
  Winuxsh host. They are not sourced as shell code and they are not blocked on
  a WASM ABI.
- `plugin list`, `plugin info`, `plugin search`, `plugin review`, and
  `plugin doctor` expose derived `execution_model`, `externalization_class`,
  and readiness profile values. These are review surfaces, not manifest schema
  fields.
- `plugin prompts`, `plugin keybindings`, and `plugin themes` expose the
  concrete asset catalogs the host can consume at runtime.
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
| `git` | `source` | Shell source plus mixed declarative/native | `.winux` startup helpers plus alias/completion assets; native git prompt segment remains host-owned | None for current first-party source/helper scope | Yes, by trusted startup source | No |
| `docker` | `source` | Shell source plus declarative asset | `.winux` helper functions plus alias/completion assets | None for current first-party source helper scope | Yes, by trusted startup source | Minimal, existing compiled aliases/completions |
| `kubectl` | `source` | Shell source plus declarative asset | `.winux` helper functions plus alias/completion assets | None for current first-party source helper scope | Yes, by trusted startup source | Minimal |
| `npm` | `source` | Shell source plus mixed declarative/native | `.winux` helper functions plus assets; native/dynamic completion remains host-owned | None for current first-party source/helper scope | Yes, by trusted startup source | No |
| `zoxide` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after cwd effects are explicit | `shell:cwd:write`, lifecycle context, rollback/failure behavior | Yes | Yes |
| `direnv` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after env writes are structured | `env:write`, scoped process adapter policy, lifecycle context, rollback/failure behavior | Yes | Yes |
| `dotenv` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after fs/env effects are structured | Scoped `fs:read`, `env:write`, lifecycle context, rollback/failure behavior | Yes | Yes |
| `fzf` | `builtin` | External-tool adapter plus shell effect | Native builtin now; future process/effect only after interactive policy | Interactive process policy, `shell:cwd:write`, rollback/failure behavior | Yes | Yes |
| `command-not-found` | `builtin` | Pure provider candidate | Host call-site and process-provider bridge are implemented; future WASM provider must use a separate ABI | `wasm_provider_abi` | No | Yes |
| `last-working-dir` | `builtin` | Shell-effect candidate | Native builtin now; future effect runtime only after cache/cwd protocol | Scoped cache read/write, startup/chpwd effect protocol, `shell:cwd:write` | Yes | Yes |
| `thefuck` | `builtin` | External-tool adapter plus shell effect | Native/process adapter plus future effect protocol; hold now | `history:read`, suggested command review/execute protocol, process adapter policy | Yes | Yes |
| `keybindings` | `builtin` | Declarative asset | Declarative bundle asset plus native Reedline action mapping | None | No | No |
| `prompts` | `builtin` | Mixed declarative/native | Declarative prompt presets plus native prompt segments | None | No | No |
| `themes` | `builtin` | Declarative asset | Declarative bundle asset plus native theme renderer | None | No | No |
| `process-echo` | `process` | Fixture | Process command fixture | None for current fixture scope | No | No |
| `process-hook` | `process` | Fixture | Process hook fixture; not a generic effect runtime | Structured hook effects before using as normal shell-mutating pack | No in fixture | No |
| `wasm-hello` | `wasm` | Fixture | WASM command fixture; provider/effect ABI is separate | Provider/effect ABI is separate future work | No | No |
## Immediate Decision Gates
1. Use `source` when the plugin's value is shell startup behavior and the
   `shell:source` trust tradeoff is acceptable.
2. Keep declarative assets separate from shell source. Prompt presets, themes,
   and keybindings are read from bundle assets and applied by Winuxsh/Reedline.
3. Do not block first-party asset packs on a new runtime kind. The current
   manifest exports plus bundle layout are sufficient; add a new asset-only
   schema only if third-party compatibility requires it.
4. Use the implemented process-provider binding around
   [command-not-found](plugin-command-not-found-provider-abi.md):
   verify missing-command input, host context, deterministic suggestion output, and fallback before migrating the official pack.
5. Keep untrusted or third-party shell-mutating packs native/source-disabled
   until effects are explicit, permissioned, reversible when possible, and
   tested against failure/timeout behavior.
