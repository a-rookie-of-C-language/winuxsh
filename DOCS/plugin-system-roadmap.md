---
tags: [winuxsh, plugins, oh-my-winuxsh, roadmap]
created: 2026-07-30
status: active
---

# Winuxsh Plugin System Roadmap

This is the execution roadmap for the Winuxsh-native plugin system and the
official `oh-my-winuxsh` bundle.

The goal is a clean open-source product line:

- Winuxsh owns the shell, config, permission model, plugin host, and update
  mechanics.
- `oh-my-winuxsh` is bundled with Winuxsh releases and can update independently.
- Existing "zsh compatibility" work is treated as migration tooling and
  first-party Winuxsh implementations, not as zsh plugin support.
- TOML remains the plugin control plane. RC remains the user script plane.
- Builtin plugins ship first. WASM follows after the host API stabilizes.
  Process/IPC remains an adapter/debug backend. DLL/FFI is not a public plugin
  ABI.

## Current State

Completed in this branch:

- `oh-my-winuxsh` reset branch created with legacy content preserved.
- Official bundle scaffold added: `bundle.toml` and `packs/*/plugin.toml`.
- Winuxsh runtime has a canonical read-only plugin registry.
- `[plugins]` TOML config parses in `~/.winshrc.toml`.
- CLI inventory, discovery, review, diagnostics, install, and bundle lifecycle
  commands exist:
  - `winuxsh plugin list [--json]`
  - `winuxsh plugin info <name> [--json]`
  - `winuxsh plugin search [query] [--json]`
  - `winuxsh plugin review <name> [--json]`
  - `winuxsh plugin doctor [--json]`
  - `winuxsh plugin install <name>`
  - `winuxsh plugin uninstall <name>`
  - `winuxsh plugin update oh-my-winuxsh --from <path>`
  - `winuxsh plugin update oh-my-winuxsh --github-release latest|vX.Y.Z`
  - `winuxsh plugin rollback oh-my-winuxsh`
- Legacy inventory remains as compatibility only:
  - `--zsh-native-packs`
  - `--zsh-native-packs-json`
- WASM command plugins have a constrained host path for modules that export
  `winuxsh_plugin_main() -> i32`. Phase 14-17 add the first host imports for
  deterministic stdout/stderr writes, simple command args, and permission-gated
  cwd/env reads without opening files, processes, env mutation, or arbitrary
  shell mutation.

## Phase 0 - Direction Lock

Status: done.

Winuxsh:

- Document that plugins are Winuxsh-native.
- Document that zsh compatibility is migration-only.
- Decide that TOML is the plugin control plane and RC is user shell code.
- Decide runtime order: `builtin` first, `wasm` later, `process` as adapter.

oh-my-winuxsh:

- Preserve legacy repository state with a branch/tag.
- Rebuild the working branch around bundle manifests, not sourced scripts.

Done when:

- Docs no longer describe `oh-my-winuxsh` as an Oh My Zsh fork.
- No plan depends on executing arbitrary `.zsh` plugin source.

## Phase 1 - Read-Only Foundation

Status: done in the current branch.

Winuxsh:

- Add runtime data model for:
  - bundle name and version;
  - plugin name, kind, category, default state;
  - permissions;
  - required binaries;
  - exports: aliases, completions, prompt segments, hooks, commands,
    keybindings.
- Register first-party builtin packs:
  - `git`;
  - `docker`;
  - `kubectl`;
  - `npm`;
  - `zoxide`;
  - `direnv`;
  - `dotenv`;
  - `fzf`;
  - `command-not-found`;
  - `last-working-dir`;
  - `thefuck`;
  - `keybindings`;
  - `prompts`.
- Add read-only CLI and tests.
- Parse `[plugins]` without changing runtime behavior yet.

oh-my-winuxsh:

- Mirror the same pack list in `bundle.toml`.
- Add one manifest per pack under `packs/<name>/plugin.toml`.
- Keep manifest fields aligned with Winuxsh registry output.

Done when:

- `cargo test --workspace --locked` passes.
- All bundle `.toml` files parse.
- `winuxsh plugin list` and `winuxsh plugin info git` work.

## Phase 2 - Managed Config Plan/Apply

Status: done in the current branch.

Winuxsh:

- Add preview commands:
  - `winuxsh plugin plan enable <pack>`;
  - `winuxsh plugin plan disable <pack>`;
  - JSON output for tooling through `--json`.
- Add apply commands:
  - `winuxsh plugin enable <pack>`;
  - `winuxsh plugin disable <pack>`.
- Write only a Winuxsh-managed TOML block.
- Create backups before writes.
- Refuse unsafe writes when:
  - the target plugin does not exist;
  - requested permissions do not match the registry;
  - user-authored `[plugins]` data would be overwritten;
  - TOML would become invalid.
- Preserve legacy `[zsh.*]` reads, but never write new canonical plugin state
  under `[zsh.*]`.

oh-my-winuxsh:

- Keep permissions in manifests audit-friendly and minimal.
- Add docs/examples for enabling and disabling packs.
- Add manifest validation so pack names in `bundle.toml` and `packs/` stay in
  sync.

Done when:

- Plan output is deterministic and reviewable.
- Apply creates a backup and writes valid TOML.
- Disabling a plugin removes or flips only the managed plugin entry.
- Re-running enable/disable is idempotent.

Implemented command surface:

```sh
winuxsh plugin plan enable git
winuxsh plugin plan disable zoxide
winuxsh plugin plan enable git --json
winuxsh plugin enable git
winuxsh plugin disable zoxide
```

## Phase 3 - Runtime Activation Bridge

Status: done on the current branch for local install/status baseline.

Winuxsh:

- Map canonical `[plugins]` state into existing builtin behavior.
- Define precedence:
  1. explicit `[plugins.<pack>]` state;
  2. `[plugins].load`;
  3. bundle defaults;
  4. legacy `[zsh.native_plugins]` / `[zsh.native_widgets]` compatibility reads.
- Make existing builtin implementations use plugin enablement:
  - dev alias packs;
  - lifecycle packs;
  - command shims;
  - keybinding presets;
  - prompt presets.
- Keep core interactive primitives outside plugin ownership:
  - history;
  - reedline integration;
  - prompt renderer;
  - syntax highlighting engine;
  - autosuggestion engine.

oh-my-winuxsh:

- Separate manifests from assets:
  - `aliases/`;
  - `completions/`;
  - `prompts/`;
  - `keybindings/`.
- Document which packs are pure metadata and which require Winuxsh builtin
  runtime support.

Done when:

- Enabling `zoxide` through `[plugins]` activates the existing builtin shim.
- Enabling `git` through `[plugins]` activates the existing builtin alias pack
  without using zsh plugin identity.
- Legacy config still works but produces migration-only language in docs/help.

Current branch progress:

- `PluginRuntimeState` resolves effective enablement from canonical `[plugins]`
  state, bundle defaults, and legacy migration-only reads.
- Explicit `[plugins.<pack>] enabled = false` overrides defaults and legacy
  reads for that pack.
- The existing git alias pack is now gated by the official `git` plugin state.
- Official builtin alias packs now use the shared compiled fallback alias table:
  `git` defaults on, while `docker`, `kubectl`, and `npm` activate only when
  enabled through canonical `[plugins]` state. User aliases remain
  authoritative on name collisions.
- Existing builtin native shims such as `zoxide`, `direnv`, `dotenv`, `fzf`,
  `thefuck`, `command-not-found`, and `last-working-dir` now check effective
  plugin state before falling back to legacy `[zsh.native_plugins]`.
- Explicitly disabling the official `keybindings` pack blocks legacy
  `[zsh.native_widgets]` presets and imported bindkey suggestions.
- Explicitly disabling the official `prompts` pack prevents segment prompt
  presets from activating while keeping the core template prompt renderer
  available.
- Tests cover default git enablement, explicit git disable, docker alias pack
  enable/disable, user alias precedence, canonical zoxide enablement, explicit
  disable overriding legacy zoxide presets, keybindings gating, and prompt
  preset gating.

## Phase 4 - Bundle Baseline Install

Status: implemented on the current branch.

Winuxsh:

- Ship a baseline copy of `oh-my-winuxsh` with releases.
- Define local paths:
  - `%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/<version>/`;
  - `%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/current`;
  - `~/.winuxsh/plugin-lock.toml`.
- Load manifests from installed bundle when available.
- Fall back to compiled builtin registry when no bundle is installed.
- Add `winuxsh plugin bundle status`.

Current branch progress:

- Defined discoverable local bundle paths and test overrides:
  - `%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/current`;
  - `%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/<version>`;
  - app-bundled `bundles/oh-my-winuxsh` next to `winuxsh.exe`;
  - `~/.winuxsh/plugin-lock.toml`;
  - `WINUXSH_PLUGIN_BUNDLE_PATH`, `WINUXSH_PLUGIN_BUNDLE_ROOT`,
    `WINUXSH_APP_BUNDLE_PATH`, and
    `WINUXSH_PLUGIN_LOCK` for tests and developer builds.
- `winuxsh plugin bundle status [--json]` reports active bundle path/version,
  lock path, fallback state, and manifest parse errors.
- `winuxsh plugin list/info` now load an installed official bundle manifest when
  available and fall back to the compiled builtin registry when absent or invalid.
- `scripts/package-release.ps1` stages `oh-my-winuxsh` into release zips and
  installer source directories by default, with an explicit skip flag for
  special builds.

oh-my-winuxsh:

- Produce a release zip with deterministic layout.
- Include `bundle.toml`, `packs/`, and static assets.
- Add checksums for release artifacts.

Done when:

- A Winuxsh release can run offline and still list official plugins.
- The installed bundle path and active bundle version are inspectable.
- Missing/corrupt bundle state falls back safely.

## Phase 5 - Bundle Update and Rollback

Status: implemented on the current branch for local and official GitHub release
updates; third-party registry provenance/signing remains future work.

Winuxsh:

- Add:
  - `winuxsh plugin update oh-my-winuxsh --from <bundle-dir-or-zip>`;
  - `winuxsh plugin rollback oh-my-winuxsh`.
- Install bundle release artifacts independently from shell self-update.
- Keep network downloads on top of the same validated local release install
  path.
- Verify:
  - official GitHub release repository and checksum asset for network downloads;
  - version;
  - checksum;
  - API compatibility;
  - `min_winuxsh` once declared by bundle manifests.
- Update `plugin-lock.toml` atomically.
- Keep previous bundle path for rollback.

Current branch progress:

- `winuxsh plugin update oh-my-winuxsh --from <path> [--checksum <sha>]`
  installs a local bundle directory or zip into the versioned bundle root.
- `--checksum-file <path>` accepts package-script `.sha256` artifacts.
- `winuxsh plugin update oh-my-winuxsh --github-release latest|vX.Y.Z`
  downloads the official `unixwin/oh-my-winuxsh` zip and `.sha256` asset before
  entering the same validated local update path.
- The runtime validates bundle name, bundle API version, `min_winuxsh`,
  manifest parseability, pack API version, and supported runtime kind before
  switching the active lock.
- `plugin-lock.toml` is written via a temporary file and rename, recording
  active path, previous path, version, and archive checksum when present.
- `winuxsh plugin rollback oh-my-winuxsh` switches the lock back to the previous
  validated bundle path without reinstalling files.
- Runtime and binary-level tests cover update, rollback, JSON output,
  checksum mismatch preserving the previously active bundle, and rejection of
  bundles that require a newer Winuxsh host.
- The runtime now validates bundle-level API
  `winuxsh:plugin-bundle@0.1.0` and refuses updates whose `min_winuxsh` is
  greater than the running `winuxsh` version.

oh-my-winuxsh:

- Publish GitHub releases with:
  - `oh-my-winuxsh-{version}.zip`;
  - checksum file;
  - changelog;
  - compatibility notes.
- Keep semver rules:
  - patch: manifest/asset fix;
  - minor: new pack or safe permission expansion;
  - major: breaking manifest/API change.
- Current bundle branch now gates releases on `CHANGELOG.md` and
  `docs/compatibility.md`, includes `CHANGELOG.md` in deterministic zips, and
  documents bundle API `winuxsh:plugin-bundle@0.1.0` plus
  `min_winuxsh = "0.8.3"`.

Done when:

- Bundle update does not require Winuxsh self-update.
- Rollback restores the previous active bundle.
- API or checksum failure leaves the old bundle active.

## Phase 6 - Move First-Party Assets Out of Core

Status: implemented on the current branch.

Winuxsh:

- Keep execution code in core for `kind = "builtin"`.
- Move data assets to the bundle when safe:
  - alias tables;
  - completion definitions;
  - prompt presets;
  - keybinding preset metadata.
- Keep compiled fallback data for offline bootstrap and corrupted bundle
  recovery.

Current branch progress:

- `oh-my-winuxsh/aliases/{git,docker,kubectl,npm}.toml` now owns first-party
  alias tables for builtin alias packs.
- Winuxsh loads aliases from the active installed bundle first and falls back to
  compiled alias tables when the bundle is absent, invalid, or missing assets.
- Bundle update validation rejects builtin packs that declare `exports.aliases`
  without a parseable non-empty aliases asset.
- Binary tests cover zip install followed by shell-visible bundle-provided
  aliases.
- `oh-my-winuxsh/completions/{git,docker,kubectl,npm}.toml` now owns static
  completion definitions for the first devtool packs. Winuxsh loads active
  bundle definitions before zsh-imported/user completion directories and keeps
  compiled definitions as fallback.
- `oh-my-winuxsh/prompts/segments.toml` now owns safe prompt segment mappings
  and preset layouts. Winuxsh loads the active bundle preset before compiled
  preset defaults, while explicit user prompt element overrides still win.
- `oh-my-winuxsh/keybindings/{common,emacs,vi}.toml` now owns declarative
  keybinding metadata only. Runtime editor actions remain native; the bundle
  does not introduce ZLE support or execute widget bodies.
- `winuxsh plugin info keybindings` surfaces active bundle keybinding metadata,
  including keymap, binding count, and bundle-owned summary text.
- Runtime tests cover bundle alias, completion, prompt preset, and keybinding
  metadata precedence; binary tests cover user-visible keybinding metadata;
  bundle validation rejects missing or malformed alias/completion/prompt and
  keybinding assets for exported builtin packs.

oh-my-winuxsh:

- Own first-party static assets.
- Add tests that every exported asset has a matching manifest entry.
- Add migration notes for changed aliases/presets.

Done when:

- Updating `oh-my-winuxsh` can update a first-party alias, completion, prompt
  preset, or keybinding metadata file without replacing `winuxsh.exe`.
- Builtin runtime behavior still has a safe fallback.

## Phase 7 - Process/IPC Backend

Status: implemented on the current branch.

Winuxsh:

- Add `kind = "process"` only for external tools that must run as separate
  processes.
- Use explicit permissions and timeouts.
- Keep process plugins out of shell parser/executor internals.
- Treat process plugins as adapter/debug bridges, not the main ecosystem.

Current branch progress:

- Added a manifest-level process contract:
  - `protocol = "winuxsh:process-plugin@0.1.0"`;
  - `command`;
  - `args`;
  - `timeout_millis`.
- Installed bundles are rejected when a process pack is default-enabled, lacks a
  valid `[process]` contract, omits `process:run:<command>`, omits the command
  from `required_binaries`, exceeds timeout bounds, or exports neither command
  nor hook.
- `plugin info` surfaces process contract metadata, and managed
  `plugin plan enable` can target process packs that exist only in the active
  installed bundle.
- Enabled process command exports run through a conservative simple-command
  path with captured stdout/stderr, deterministic exit status, and timeout
  handling.
- Enabled process lifecycle hook exports run at startup, precmd, preexec, and
  chpwd hook points with context passed through environment variables.

oh-my-winuxsh:

- Added `process-echo` and `process-hook` as explicit opt-in process manifest
  fixtures.

Done when:

- A process plugin can expose a command or hook through the same manifest model.
- Failure, timeout, stderr, and exit status are deterministic.

## Phase 8 - WASM/WASI Backend

Status: minimal command execution implemented on the current branch; full WASI/component host remains future work.

Winuxsh:

- Define a stable host API before adding third-party WASM.
- Prefer WASM Component Model/WIT-style interfaces if the toolchain is stable
  enough.
- Start with non-invasive capabilities:
  - completions;
  - prompt segment calculation;
  - command suggestions;
  - pure transforms.
- Add stateful or shell-mutating capabilities later:
  - env writes;
  - cwd changes;
  - lifecycle hooks.
- Enforce permission prompts, deterministic IO, and versioned ABI.

Current branch progress:

- Added manifest-level WASM contract metadata:
  - `protocol = "winuxsh:wasm-plugin@0.1.0"`;
  - `module`;
  - `sha256`;
  - `wit_world`;
  - `timeout_millis`;
  - `max_memory_pages`.
- Installed bundles are rejected when a WASM pack is default-enabled, lacks a
  valid `[wasm]` contract, declares native required binaries, requests
  `process:run:*`, exports lifecycle hooks, exceeds timeout/memory bounds, or
  points outside the bundle.
- WASM module artifacts are read from the bundle, checked against manifest
  SHA-256, and validated as WASM binaries before a bundle can become active.
- `plugin info` surfaces WASM contract metadata for installed bundle packs.
- Enabled WASM command exports now run through a conservative simple-command
  path. Command modules must export `winuxsh_plugin_main() -> i32`; that
  return value becomes the shell exit status.
- The current sandbox path instantiates modules with wasmi, disables hidden
  start functions, limits memory/instances/tables from manifest caps, and uses
  fuel metering to deterministically stop runaway modules with exit code 124.
- Phase 14 adds a minimal `winuxsh:plugin/host` IO ABI:
  - `stdout_write(ptr: i32, len: i32) -> i32`;
  - `stderr_write(ptr: i32, len: i32) -> i32`.
- Phase 15-17 extend that ABI with simple command args plus permission-gated
  cwd and env reads:
  - `arg_count() -> i32`;
  - `arg_len(index: i32) -> i32`;
  - `arg_read(index: i32, ptr: i32) -> i32`;
  - `cwd_len() -> i32`;
  - `cwd_read(ptr: i32) -> i32`;
  - `env_len(name_ptr: i32, name_len: i32) -> i32`;
  - `env_read(name_ptr: i32, name_len: i32, value_ptr: i32) -> i32`.
- Host IO reads bytes from exported module memory, buffers stdout/stderr, and
  returns `-1` for missing memory, invalid pointers, out-of-bounds reads, or
  writes over the per-stream 64 KiB cap.
- `cwd_len` and `cwd_read` return `-1` unless the pack declares `cwd:read`.
- `env_len` and `env_read` return `-1` unless the pack declares a matching
  `env:read:<NAME>` permission.
- Tests cover successful `wasm-hello` command execution, deterministic
  failure when the required main export is missing, out-of-fuel termination
  with exit code 124 for runaway modules, stdout/stderr host writes,
  missing-memory host write rejection, command argv reads, permission-gated cwd
  reads, denied cwd reads, permission-gated env reads, and denied env reads.

oh-my-winuxsh:

- Keep official first-party packs builtin unless WASM has a clear distribution
  benefit.
- Added `wasm-hello` as an explicit opt-in host API fixture, not as a required
  runtime for current builtins.

Done when:

- A third-party WASM command plugin can be installed, listed,
  permission-checked, and executed without trusting native code.
- WASM failure cannot crash the shell.

## Phase 9 - Open Ecosystem

Status: implemented for the official bundle authoring/discovery surface on the current branch; broader third-party registry trust remains future work.

Winuxsh:

- Add plugin discovery/install after bundle update is reliable.
- Add:
  - signing or checksum policy;
  - package index format;
  - permission review UX;
  - compatibility matrix;
  - `winuxsh plugin doctor`.

Current branch progress:

- Added `winuxsh plugin search [query] [--json]` as the read-only discovery surface over the active official inventory.
- Added `winuxsh plugin doctor [--json]` as a read-only diagnostic surface.
- The doctor reports active bundle state, enabled packs, missing required
  binaries, configured-vs-manifest permission drift, unknown configured packs,
  and bundle candidate warnings without mutating config or executing plugin
  commands.
- Added `winuxsh plugin review <pack> [--json]` as the permission review UX.
  It explains each manifest permission token, runtime kind, exported surfaces,
  trust source, required binaries, current enablement, and the exact enable
  command without mutating config or executing plugin commands.
- `winuxsh plugin list` / `plugin info` / `plugin search` / bundle status /
  doctor expose bundle source and trust source, so local overrides and future
  external bundles do not look like first-party official packs.
- Bundle status uses trust-aware titles and messages; external bundle overrides
  report as review-only instead of "installed official bundle."
- External bundle packs are review-only until third-party registry trust policy
  lands; `plugin install` refuses to write managed config for them instead of
  silently recording them as `oh-my-winuxsh` packs.
- Tests cover text/JSON plugin search, plugin install, JSON output for an
  enabled WASM pack, text warnings for a process pack with a missing required
  binary, text/JSON permission review output for process and WASM packs, and
  explicit `official_bundle` / `local_override` / `external_bundle` discovery,
  status, doctor, and review output, plus user-visible bundle keybinding
  metadata inspection.

oh-my-winuxsh:

- Become the reference bundle implementation.
- Document pack authoring conventions.
- Provide templates and validation scripts.

Current branch progress:

- oh-my-winuxsh now includes `docs/authoring.md` with manifest schema,
  permission token descriptions, runtime contracts, asset rules, and a release
  checklist.
- The bundle ships copyable `templates/builtin`, `templates/process`, and
  `templates/wasm` manifests.
- `tools/validate_bundle.py` validates the authoring guide, templates, package
  index, release metadata, and CI workflow so the public authoring surface is
  mechanically checked.
- Deterministic bundle packaging includes `templates/` in release artifacts.
- `.github/workflows/bundle.yml` runs py_compile, bundle validation, and
  deterministic package checks on push and pull requests.

Done when:

- A third-party author can publish a plugin without imitating zsh or Oh My Zsh.
- Users can audit what a plugin can read, write, or execute before enabling it.

## Phase 10 - Legacy Cleanup

Status: implemented on the current branch.

Winuxsh:

- Keep zsh migration commands, but move old inventory wording to legacy help.
- Stop documenting `[zsh.native_plugins]` and `[zsh.native_widgets]` as primary
  config.
- Keep compatibility reads for at least one stable release after `[plugins]`
  runtime activation is shipped.

Current branch progress:

- CLI help now labels `--zsh-native-packs` and `--zsh-native-packs-json` as
  legacy zsh migration inventory.
- The legacy text inventory says migration pack mappings instead of branding a
  old zsh-native product surface.
- README and getting started docs point users to `winuxsh plugin list/search`,
  `winuxsh plugin review`, `winuxsh plugin install`, and `winuxsh plugin uninstall`.
- Getting started now describes `[plugins]` as the canonical TOML surface and
  confines `[zsh.native_plugins]` / `[zsh.native_widgets]` to legacy migration
  compatibility wording.
- CLI help labels `--zsh-compat-doctor` as a legacy zsh migration health check.

oh-my-winuxsh:

- Keep legacy branch/tag available.
- Do not reintroduce sourced script plugins.

Done when:

- New docs and examples use only `winuxsh plugin ...` and `[plugins]`.
- zsh is mentioned only under migration.

## Phase 11 - Theme Pack Assets

Status: implemented on the current branch as the official-bundle foundation;
third-party theme marketplace/distribution remains future work.

Winuxsh:

- Extend plugin manifests with `exports.themes`.
- Validate exported theme assets during official bundle install/update.
- Load active official bundle themes after built-in themes and user theme files,
  preserving user-authored theme overrides for non-built-in names.
- Surface active bundle theme names in the setup wizard.

Current branch progress:

- Added `exports.themes` to the host manifest model, JSON inventory, permission
  review text, plugin info text, and search matching.
- Added active bundle theme loading through the existing Winuxsh theme TOML
  schema.
- Added regression coverage for a temporary installed bundle exporting
  `themes/ocean.toml`.

oh-my-winuxsh:

- Add a default-on `themes` builtin UX pack.
- Ship official `cyberpunk`, `forest`, `minimal`, and `ocean` theme assets.
- Validate theme files in `tools/validate_bundle.py` and include `themes/` in
  deterministic package artifacts.

Done when:

- A bundle release can add or adjust official themes without replacing
  `winuxsh.exe`.
- Missing or invalid exported theme assets fail bundle validation before install.

## Phase 12 - Registry Trust Policy Foundation

Status: implemented on the current branch for official bundle index validation;
third-party registry signing remains future work.

Winuxsh:

- Validate installed bundle `index.toml` before switching `plugin-lock.toml`.
- Require index bundle/version/API/min-host fields to match `bundle.toml`.
- Require index pack entries to match shipped `packs/<name>/plugin.toml`
  manifests for version, API, kind, category, summary, default state,
  permissions, and required binaries.
- Enforce release policy: `checksum_required = true`,
  `checksum_algorithm = "sha256"`, and `signature = "unsupported"` until
  signing support exists.
- Reject checksum-required zip updates that omit `--checksum` or
  `--checksum-file`, leaving the active bundle untouched.

oh-my-winuxsh:

- Treat `oh-my-winuxsh` as the official first-party bundle and reference layout,
  not the only future plugin location.
- Publish index policy explicitly so future third-party registries can bridge
  into Winuxsh through the same host-side validation contract.

Done when:

- Zip update without checksum fails before lock switching.
- Index/manifest drift fails before install.
- Unsupported signatures are explicit in the release index.

## Phase 13 - Theme Market Discovery Surface

Status: implemented on the current branch as a read-only catalog foundation;
theme install/apply marketplace commands remain future work.

Winuxsh:

- Add `winuxsh plugin themes [--json]` as the read-only discovery surface for
  built-in themes, user theme TOML files, and active official bundle theme
  exports.
- Expose catalog `trust_source` values so bundle, local override, external
  bundle, user theme, and built-in sources stay auditable in text and JSON
  output.
- Preserve resolution order explicitly as built-in > user > active bundle, so
  user-authored themes remain safe overrides for bundle-provided names.
- Include valid `~/.winuxsh/themes/*.toml` entries in theme discovery and setup
  theme name enumeration without changing the config file.
- Validate active bundle theme assets before listing them in the catalog.

oh-my-winuxsh:

- Keep official theme assets under the `themes` pack as the first catalog data
  source.
- Document `winuxsh plugin themes` as the bridge between the first-party bundle
  and future third-party theme distribution.

Done when:

- `winuxsh plugin themes` lists built-in and active bundle theme sources.
- `winuxsh plugin themes --json` exposes machine-readable source, owner, pack,
  path, and trust-source metadata.
- No command in this phase installs or selects a theme automatically.

## Phase 14 - WASM Host IO ABI Foundation

Status: implemented on the current branch as the first constrained host ABI;
full WASI/component host support remains future work.

Winuxsh:

- Register `winuxsh:plugin/host` imports for `stdout_write` and
  `stderr_write` before instantiating enabled WASM command packs.
- Require plugin output bytes to come from the module's exported `memory` and
  keep the existing manifest memory/fuel sandbox.
- Buffer stdout/stderr inside the store state and flush after module execution,
  including trapped executions that wrote before failing.
- Return `-1` for unsafe host writes instead of trapping the shell.

oh-my-winuxsh:

- Keep `wasm-hello` as an explicit opt-in host contract fixture.
- Document that host IO exists, while files, processes, env mutation, lifecycle
  hooks, completions, prompt segments, WASI, and shell mutation remain outside
  the current public WASM contract.

Done when:

- A WASM command fixture can write stdout and stderr through the host ABI.
- A WASM command without exported memory receives a deterministic `-1` host
  write result.
- Existing no-output WASM command, missing export, and out-of-fuel behavior
  remains unchanged.

## Phase 15 - WASM Command Args ABI Foundation

Status: implemented on the current branch as a constrained command-argument
host ABI; full WASI/component argv/env support remains future work.

Winuxsh:

- Allow enabled WASM command exports to receive simple-command arguments instead
  of rejecting all argv after the command name.
- Register `arg_count() -> i32`, `arg_len(index: i32) -> i32`, and
  `arg_read(index: i32, ptr: i32) -> i32` on `winuxsh:plugin/host`.
- Keep argument bytes host-owned until a module explicitly copies one argument
  into exported module memory.
- Return `-1` for invalid indexes, missing memory, out-of-bounds writes, or
  arguments over the 64 KiB per-argument cap.

oh-my-winuxsh:

- Document command args as part of the current WASM public contract.
- Keep WASI, env mutation, file/process access, lifecycle hooks, completions,
  prompt segments, keybindings, and shell mutation outside the current contract.

Done when:

- A WASM command fixture can read two command arguments and echo them through
  host stdout.
- Existing no-argument, host IO, missing export, and out-of-fuel behavior remains
  unchanged.
## Phase 16 - WASM Cwd Read ABI Foundation
Status: implemented on the current branch as a permission-gated cwd read host
ABI; files, env mutation, process access, WASI, and shell mutation remain future
work.
Winuxsh:
- Register `cwd_len() -> i32` and `cwd_read(ptr: i32) -> i32` on
  `winuxsh:plugin/host`.
- Expose the shell-visible `PWD` only when the pack declares `cwd:read`.
- Copy cwd bytes from host state into exported module memory only after the
  module calls `cwd_read`.
- Return `-1` for missing `cwd:read`, missing memory, invalid pointers,
  out-of-bounds writes, or cwd values over the 64 KiB cap.
oh-my-winuxsh:
- Document cwd reads as part of the current WASM public contract when gated by
  `cwd:read`.
- Keep WASI, env mutation, file/process access, lifecycle hooks, completions,
  prompt segments, keybindings, and shell mutation outside the current contract.
Done when:
- A WASM command fixture can read the shell-visible current directory and echo
  it through host stdout when `cwd:read` is declared.
- A WASM command fixture receives deterministic `-1` results from `cwd_len` and
  `cwd_read` without `cwd:read`.
- Existing host IO, command args, missing export, and out-of-fuel behavior
  remains unchanged.
## Phase 17 - WASM Env Read ABI Foundation
Status: implemented on the current branch as a permission-gated env read host
ABI; env mutation, files, process access, WASI, and shell mutation remain future
work.
Winuxsh:
- Register `env_len(name_ptr: i32, name_len: i32) -> i32` and
  `env_read(name_ptr: i32, name_len: i32, value_ptr: i32) -> i32` on
  `winuxsh:plugin/host`.
- Expose only environment variables whose names are explicitly declared through
  `env:read:<NAME>` permissions.
- Copy env value bytes from host state into exported module memory only after
  the module calls `env_read`.
- Accept ASCII alphanumeric and `_` environment variable names up to 256 bytes.
- Return `-1` for missing matching permissions, missing memory, invalid env
  names, invalid pointers, out-of-bounds writes, missing values, or values over
  the 64 KiB cap.
oh-my-winuxsh:
- Document env reads as part of the current WASM public contract when gated by
  matching `env:read:<NAME>` permissions.
- Keep WASI, env mutation, file/process access, lifecycle hooks, completions,
  prompt segments, keybindings, and shell mutation outside the current contract.
Done when:
- A WASM command fixture can read an allowed host environment variable and echo
  it through host stdout when `env:read:<NAME>` is declared.
- A WASM command fixture receives deterministic `-1` results from `env_len` and
  `env_read` without the matching `env:read:<NAME>` permission.
- Existing host IO, command args, cwd reads, missing export, and out-of-fuel
  behavior remains unchanged.
## Phase 18 - Command-Not-Found Provider ABI Skeleton
Status: implemented on the current branch as host-side request, output parsing,
and fallback helpers. Process process binding in tests and preserves compiled fallback behavior.is tracked separately in Phase 19; WASM provider runtime remains future work.
Winuxsh:
- Represent the command-not-found provider input as missing command, optional
  args, optional cwd, and available package-search helpers.
- Parse provider output as bounded UTF-8 suggestion lines separate from command
  stdout.
- Treat empty output, invalid UTF-8, oversized output, and provider runtime
  failure as fallback to compiled native hints.
- Preserve the base command-not-found diagnostic before provider or fallback
  suggestions.
- Keep current runtime behavior on the compiled native implementation until a
  provider invocation path is proven and the official bundle chooses to migrate.
oh-my-winuxsh:
- Keep the command-not-found pack marker as exports.providers =
  ["command-not-found"].
- Do not ship a process or WASM provider replacement until Winuxsh proves the
  process binding in tests and preserves compiled fallback behavior.
Done when:
- Unit tests cover provider request context, valid suggestions, empty output,
  failure fallback, invalid output fallback, and native hint preservation.
- Existing command-not-found native hints remain unchanged.
## Phase 19 - Command-Not-Found Process Provider Binding
Status: implemented on the current branch as a process-provider binding for command-not-found; WASM provider ABI remains future work.
Winuxsh:
- Allow process packs to export command-not-found when command:diagnose is declared.
- Invoke enabled process providers from command mode through the rubash host external hook after aliases, functions, builtins, and real PATH commands resolve.
- Pass missing command, args, optional cwd, and package helper availability through host-owned argv/env.
- Suppress provider stderr from user diagnostics and fall back to compiled native hints on timeout, failure, invalid output, empty output, or nonzero exit.
oh-my-winuxsh:
- Keep the official command-not-found pack builtin until the bundle intentionally ships a provider implementation.
- Do not introduce a new runtime kind for this; process provider binding uses existing kind = process plus provider exports.
Done when:
- Focused process-provider tests cover suggestion output, stderr suppression, timeout fallback, and command-mode integration.
- Provider export validation still rejects WASM provider exports until a WASM provider ABI exists.

## Future Direction - Builtin Externalization
Status: proposed direction; do not treat the current `builtin` registry as the
final plugin architecture.
The current official bundle is intentionally TOML-heavy because many first-party
packs are declarations over Winuxsh-owned code plus static bundle assets. This
is acceptable as a transition layer, but future plugin work should make room
for code-bearing packs through `wasm` and `process` runtimes.
The next step is not another runtime phase by itself; it is the readiness gate
in [Plugin Externalization Readiness](plugin-externalization-readiness.md),
which classifies every official pack before schema or runtime migration work.
Winuxsh:
- Keep `~/.winshrc` as user-owned trusted shell code. Do not replace personal
  rc scripting with WASM.
- Use WASM for distributable plugin code that should run with an explicit host
  ABI, manifest permissions, memory limits, timeouts, and deterministic IO.
- Use process plugins for adapters around existing native executables or
  interactive tools.
- Add host APIs before migrating builtin behavior. Required future surfaces may
  include completion/provider output, prompt segment output, scoped file reads,
  env writes, cwd writes, lifecycle hook context, and safe failure rollback.
- Keep rubash parsing/execution, reedline primitives, Windows path/cwd/env
  synchronization, and core shell contract helpers inside Winuxsh core.
oh-my-winuxsh:
- Continue to own static assets directly: aliases, completion definitions,
  prompt presets, keybinding metadata, and themes.
- Keep asset/declarative packs separated conceptually from host-owned builtin
  behavior while Winuxsh decides whether that becomes a new kind, an
  execution marker, or another schema field.
- Carry WASM artifacts or process manifests only when the matching Winuxsh host
  API is stable enough to validate, review, enable, run, and roll back safely.
- Do not add sourced rc plugin scripts to make the bundle resemble Oh My Zsh.
Good early candidates for externalization are pure or near-pure providers. The
first draft target is
[command-not-found](plugin-command-not-found-provider-abi.md); prompt segment
calculators, completion providers, and formatter/suggestion helpers can follow
after that provider shape is proven. Packs that mutate shell state, such as
`zoxide`, `dotenv`, `direnv`, `fzf`-cd, or lifecycle hook adapters, must wait
for explicit permissioned host APIs.
Done when:
- At least one non-fixture first-party pack moves from `builtin` to WASM or
  process without regressing permission review, deterministic command behavior,
  or rollback.
- Static TOML assets remain useful independently from executable plugin code.
- User rc remains separate from distributed plugin execution.
## Naming Rules
Use:

- "Winuxsh plugin";
- "official bundle";
- "`oh-my-winuxsh/<pack>`";
- "zsh migration";
- "keybinding preset".

Do not use for new product surfaces:

- "zsh plugin support";
- "Oh My Zsh compatibility layer";
- "standard ZLE";
- "native zsh pack" except in legacy migration notes.

## Cross-Repository Sync Rule

Every plugin phase must update both repositories:

- Winuxsh changes the host, config, CLI, tests, and release behavior.
- `oh-my-winuxsh` changes bundle metadata, pack manifests, assets, release
  notes, and compatibility docs.

A phase is not complete until both sides are aligned.
