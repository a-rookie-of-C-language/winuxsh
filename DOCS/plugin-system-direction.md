# Plugin System Direction

This note is the authoritative direction for the Winuxsh plugin system.
The execution sequence lives in [Plugin System Roadmap](plugin-system-roadmap.md).

Winuxsh plugins are Winuxsh-native. They are not zsh plugins and not an
Oh My Zsh compatibility layer. The plugin system can now load reviewed,
bundle-local `.winux` source scripts for Oh My-style shell customization, but
it must not source arbitrary `.zsh`, legacy `.winsh`, or user-discovered rc
fragments as plugin code. The old zsh migration pack mappings are treated as
the first batch of Winuxsh-owned features that now live behind a real plugin
registry and an official bundled distribution.

## Decision

- Build a built-in Winuxsh plugin system.
- Ship `oh-my-winuxsh` as the official bundled plugin distribution, similar in
  spirit to how winuxcmd is bundled as the default coreutils layer.
- Keep `~/.winuxshrc` as the primary interactive user entry point for plugin
  lists, theme selection, prompt templates, `export`, `alias`, functions, and
  startup logic.
- Keep `~/.winshrc` as a compatibility fallback only when `~/.winuxshrc` is
  absent.
- Keep `~/.winshrc.toml` as legacy/managed structured state for plugin CLI
  enablement, permissions, bundle versions, migration blocks, tests, and
  advanced machine-editable overrides.
- Keep zsh compatibility as a one-time migration/onboarding tool only. It may
  detect familiar `.zshrc` intent and suggest Winuxsh plugins, but it must not
  define the plugin system's identity.
- Treat the current `builtin` packs as a first-party transition layer, not as
  the final definition of what a plugin can be. Oh My-style aliases,
  functions, and startup helpers should migrate into bundle-local `.winux`
  source packs guarded by `shell:source`; sandboxed providers and commands
  should move through the same manifest and permission model using `wasm` or
  `process` runtimes.
- Do not support ZLE. Only a small set of zsh-style keybinding names may be
  translated to native reedline editor actions.

## Product Model

```text
winuxsh core
  - rubash shell execution
  - reedline interactive frontend
  - config loading
  - plugin registry
  - permission model
  - bundle update / rollback plumbing

oh-my-winuxsh bundled distribution
  - official first-party plugin manifests
  - .winux source scripts, aliases, completions, prompt presets, keybinding presets
  - builtin adapters for existing native Winuxsh features
  - independent version and update channel

external plugins
  - source plugins for reviewed, trusted shell startup code
  - wasm plugins as the preferred long-term third-party runtime
  - process plugins as a compatibility/debug bridge
```

## TOML, RC, and Code

The plugin model intentionally separates declaration from execution:

- `~/.winuxshrc` is the user-owned trusted script plane and the default
  interactive entry point. Users can write bash-like shell code there: plugin
  arrays, theme selection, aliases, exports, functions, and local startup
  behavior.
- `~/.winshrc` remains a legacy compatibility fallback for older user setups.
- `~/.winshrc.toml` is the legacy/managed structured file. It records plugin
  CLI decisions, permissions, bundle versions, migration blocks, tests, and
  advanced overrides when a machine-editable format is required.
- `oh-my-winuxsh` is a distributable bundle. It can carry static assets
  directly, and it can carry code-bearing artifacts only through explicit
  runtime contracts: `.winux` source scripts, `process`, or `wasm`.

TOML is not meant to replace bash syntax. It cannot and should not express
arbitrary shell behavior. A pack that only needs aliases, completions, prompt
presets, keybinding metadata, or themes can be pure TOML. A pack that needs
shell-language behavior can be a `source` pack with a bundle-local
`packs/<name>/init.winux` entry. A pack that needs native process isolation can
use `process`; a pack that needs sandboxed deterministic code should use
`wasm` as that ABI matures. Native host-only features remain `builtin`.

This deliberately borrows the useful part of zsh and bash plugin ecosystems:
small startup scripts can mutate the current shell by being sourced. The
Winuxsh boundary is stricter: source plugin scripts must be declared in the
manifest, live inside the bundle, use the `.winux` suffix, and request the
high-risk `shell:source` permission before they are enabled.

## Runtime Kinds

Plugins share one manifest and one host API model, but may have different
execution backends:

| kind | Purpose | Stability |
| --- | --- | --- |
| `builtin` | Existing Rust implementations shipped inside winuxsh | Native fallback / host-owned features |
| `source` | Bundle-local `.winux` shell startup scripts sourced into the current interactive session | First-class Oh My-style runtime |
| `wasm` | Third-party and distributable plugins through WASM/WASI | Long-term target |
| `process` | External tool bridge, debugging, and compatibility adapters | Supported but not the main ecosystem |

This means source, IPC/process, and WASM plugins are mutually exclusive at the
single plugin instance level, but not at the architecture level. They are just
runtime backends behind the same plugin contract.

Source plugins run during interactive startup and the `-C` one-shot REPL path.
When `~/.winuxshrc` exists, it is the source-plugin/framework entry point and
loads the framework directly. Without `~/.winuxshrc`, the legacy managed path
can still load enabled source plugins before fallback `~/.winshrc`. Ordinary
`winuxsh -c`, script files, and stdin script execution stay quiet and do not
load source plugins.

WASM does not replace source plugins or `~/.winuxshrc`. The rc file remains the
user's personal trusted script surface. Source plugins are the reviewed shell
customization surface. WASM is for distributable plugin code that should run
under a host contract with explicit permissions, resource limits, and a stable
ABI. In other words, rc is the user's freedom surface; source is the Oh
My-style bundle surface; WASM is the sandbox/provider surface.

The current WASM command ABI is intentionally small: command modules export
`winuxsh_plugin_main() -> i32`, can write deterministic stdout/stderr through
host imports, can read simple command arguments, and can read cwd/env values
only when the matching `cwd:read` or `env:read:<NAME>` permissions are declared.
Files, process access, env mutation, cwd mutation, lifecycle hooks, completions,
prompt segments, and shell mutation remain future host contracts.

Before changing plugin schema or moving official packs out of `builtin`, use
[Plugin Externalization Readiness](plugin-externalization-readiness.md) as the
per-pack decision gate. That matrix must drive whether the eventual schema adds
a new runtime kind, keeps `kind = "builtin"` with an execution marker, or uses
another asset-only classification.

## Control Plane vs Script Plane

TOML stays because managed plugin operations need declaration, auditability,
rollback, and permission review:

```toml
[plugins]
enabled = true
bundles = ["oh-my-winuxsh"]
load = ["git", "zoxide", "keybindings"]

[plugins.git]
enabled = true
permissions = ["shell:source", "cwd:read", "process:run:git"]

[plugins.zoxide]
enabled = false
permissions = ["cwd:read", "process:run:zoxide"]
```

Use `winuxsh plugin review <pack> [--json]` before enabling a pack to inspect
its manifest permissions, runtime backend, required binaries, exported surfaces,
trust source, and current enablement without running plugin code or writing
config.

`~/.winuxshrc` is the normal free-form shell script:

```sh
export EDITOR=vim
alias ll='ls -la'

hello() {
  echo "hello from winuxsh"
}
```

The rule is simple: `~/.winuxshrc` is the human-authored interactive entry
point; TOML is legacy/managed machine state. Plugin permission grants, bundle
versions, and update locks should remain auditable without executing arbitrary
user rc code, but normal users should not need to edit TOML just to choose a
theme or load first-party plugins.

## oh-my-winuxsh Bundle

`oh-my-winuxsh` should be rebuilt as the official bundle repository. It should
not inherit old zsh compatibility semantics.

Repository transition:

- preserve history;
- tag or branch the old content as legacy;
- rebuild `main` around official Winuxsh plugin bundle manifests;
- document that zsh migration is only an onboarding signal, not plugin identity.

Suggested bundle layout:

```text
oh-my-winuxsh/
  bundle.toml
  packs/
    git/plugin.toml
    git/init.winux
    docker/plugin.toml
    docker/init.winux
    kubectl/plugin.toml
    npm/plugin.toml
    zoxide/plugin.toml
    direnv/plugin.toml
    dotenv/plugin.toml
    fzf/plugin.toml
    command-not-found/plugin.toml
    thefuck/plugin.toml
    keybindings/plugin.toml
    prompts/plugin.toml
  completions/
  aliases/
  prompts/
  docs/
```

The bundle should expose a public authoring surface through `docs/authoring.md`
and copyable manifest templates. Winuxsh remains the host-side auditor through
`winuxsh plugin doctor [--json]` and `winuxsh plugin review <pack> [--json]`.

The bundle should support independent versioning and update/rollback state:

```text
%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/<version>/
%LOCALAPPDATA%/Winuxsh/bundles/oh-my-winuxsh/current
~/.winuxsh/plugin-lock.toml
```

`winuxsh --self-update` updates the shell. The plugin command updates local
bundle release artifacts independently:

```sh
winuxsh plugin update oh-my-winuxsh --from dist\oh-my-winuxsh-1.0.0.zip --checksum-file dist\oh-my-winuxsh-1.0.0.zip.sha256
winuxsh plugin rollback oh-my-winuxsh
```

## Existing Feature Migration

Do not rewrite existing features into WASM first. Move shell-language helpers
behind the plugin registry as `kind = "source"` where `.winux` startup code is
the natural fit, and keep native host behavior as `kind = "builtin"` where
Winuxsh still owns the implementation.

Keep as core interactive features:

- autosuggestions;
- syntax highlighting;
- prompt renderer;
- history and reedline primitives.

Move into the bundled plugin registry as first-party packs:

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

This is a transitional product shape. It is acceptable for a first-party pack
to be `kind = "builtin"` while the host APIs are still young, but builtin
should not become the only long-term plugin model. Builtin packs should be
reviewed periodically and split into `source`, `process`, or `wasm` runtime
packs when doing so improves distribution, auditability, or third-party
extensibility.

Candidate migration groups:

| Group | Direction |
| --- | --- |
| Static assets: aliases, completion tables, themes, prompt presets, keybinding metadata | Keep in `oh-my-winuxsh` as TOML assets. No code runtime required. |
| Oh My-style shell helpers: aliases, functions, exports, startup glue | Use `kind = "source"` with a bundle-local `init.winux` and `shell:source`. |
| Pure providers: command-not-found suggestions, prompt segment calculation, completion providers, formatters | Prefer future `wasm` where the plugin is mostly input -> output. |
| External-tool adapters: `thefuck`, `direnv`, `fzf`-style launchers | Prefer `process` when native process behavior or interactive tools are the point. |
| Shell-mutating helpers: `zoxide`, `dotenv`, lifecycle hooks, env/cwd changes | Use `source` only when trusted shell mutation is the product requirement; use future effect APIs for sandboxed/third-party mutation. |
| Core shell machinery: rubash parser/executor, reedline primitives, Windows cwd/env/path synchronization, native builtins | Keep in Winuxsh core. Do not externalize just to make the bundle appear more code-rich. |

Migration order:

1. Classify every official pack in the readiness matrix before changing schema.
2. Define the host API first.
3. Add a small fixture pack that proves permissions, errors, timeout, and
   output behavior.
4. Migrate one low-risk builtin pack at a time.
5. Keep a compiled fallback until the externalized pack is stable across at
   least one release.
5. Preserve user rc as a separate personal scripting surface throughout.

Deprecate names and docs that imply zsh plugin support:

- `native zsh packs`;
- `zsh.native_plugins`;
- `zsh.native_widgets`;
- `standard ZLE widgets`.

Use:

- `official Winuxsh plugins`;
- `[plugins]`;
- `keybinding presets`;
- `zsh migration`.

## Minimum Manifest Surface

```toml
name = "git"
bundle = "oh-my-winuxsh"
version = "1.0.0"
kind = "source"
api = "winuxsh:plugin@0.1.0"
summary = "Git aliases, completions, and prompt segments."
permissions = ["shell:source", "cwd:read", "process:run:git"]

[exports]
aliases = true
completions = ["git"]
prompt_segments = ["git"]
hooks = ["startup"]
commands = []

[source]
entry = "packs/git/init.winux"

[settings]
show_dirty = true
show_ahead_behind = true
```

## Implementation Order

Use [Plugin System Roadmap](plugin-system-roadmap.md) as the active execution
plan. The short version is:

1. finish managed TOML plan/apply for enable/disable;
2. bridge `[plugins]` into current builtin runtime behavior;
3. install and load the bundled `oh-my-winuxsh` baseline;
4. add independent bundle update/rollback and `plugin-lock.toml`;
5. move first-party data assets into the bundle where safe;
6. add `source` runtime for reviewed bundle-local `.winux` shell startup code;
7. add `process` only as an adapter/debug backend;
8. add `wasm` after the host API is stable.

## Non-goals

- No zsh plugin runtime.
- No ZLE runtime.
- No arbitrary `source plugin.zsh`, legacy `.winsh`, or user-discovered rc
  fragments as plugin code.
- No plugin access to rubash parser/executor internals.
- No DLL/FFI plugin ABI for community plugins.
- No plugin behavior that depends on executing legacy fallback `~/.winshrc`.
