---
tags: [winuxsh, roadmap, v3, design, plugins]
created: 2026-07-17
updated: 2026-07-30
status: draft
---

# Winuxsh v3 Design Plan

> Goal: build a Winuxsh-native plugin system without weakening the locked shell
> architecture.

## Locked Architecture

- Shell language semantics stay in `rubash`.
- Coreutils stay in `winuxcmd.exe` through PATH injection and command links.
- REPL/frontend behavior stays in `reedline`.
- Config remains backward compatible with `~/.winshrc.toml`.
- User shell startup code remains in `~/.winshrc`.
- History remains `~/.winuxsh_history`.
- `~` and default config/history locations map to the normal Windows user home,
  not to an isolated Unix environment.
- Winuxsh inherits and mutates the real Windows process environment.
- Do not restore the old winsh lexer/parser/core/ast stack.
- Do not reintroduce winuxcmd FFI/DLL integration.
- Do not adopt Nushell syntax, pipeline semantics, or data model.
- Do not duplicate rubash-owned builtins or job-control semantics in winuxsh.

## v3 Direction

Winuxsh plugins are Winuxsh-native. They are not zsh plugins and not an
Oh My Zsh compatibility runtime.

The v3 plugin direction has four layers:

1. `winuxsh core`: config, plugin registry, permission model, bundle update and
   rollback.
2. `oh-my-winuxsh`: official bundled plugin distribution shipped with winuxsh.
3. `builtin` packs: existing Rust implementations registered through manifests.
4. External runtime backends: `wasm` as the long-term third-party path,
   `process` as an adapter/debug bridge.

## Config Boundary

`~/.winshrc.toml` is the plugin control plane:

```toml
[plugins]
enabled = true
bundles = ["oh-my-winuxsh"]
load = ["git", "prompts", "keybindings"]

[plugins.git]
enabled = true
permissions = ["cwd:read", "process:run:git"]
```

`~/.winshrc` is the user script plane:

```sh
export EDITOR=vim
alias ll='ls -la'
```

Do not collapse TOML into rc. RC has more freedom because it is shell code, but
plugin state needs deterministic, auditable, machine-editable configuration.

## Plugin Contract

Define plugin goals before choosing runtime mechanics:

- add completion providers;
- add prompt segments and prompt presets;
- add lifecycle hooks: `precmd`, `preexec`, `chpwd`;
- add helper commands;
- expose safe metadata about cwd, env, aliases, last exit code, and command
  words;
- return declarative actions such as `set-env`, `print`, `change-cwd`, or
  `completion-candidates`.

No plugin model may extend rubash parser or executor internals.

## Runtime Kinds

| kind | Purpose | v3 role |
| --- | --- | --- |
| `builtin` | First-party Rust implementation in winuxsh | First implementation target |
| `wasm` | WASM/WASI component plugin | Long-term third-party target |
| `process` | External executable over IPC/stdio | Compatibility and debugging bridge |

IPC and WASM should not become separate plugin ecosystems. They are runtime
backends behind the same manifest and host API.

## Oh My Winuxsh

`oh-my-winuxsh` should be rebuilt as the official bundled plugin repository:

- preserve the old repository history by tagging/branching legacy content;
- rebuild `main` as the Winuxsh plugin bundle;
- ship a baseline copy with winuxsh releases;
- support independent versioning, update, checksum validation, and rollback;
- store active bundle state in `~/.winuxsh/plugin-lock.toml` or equivalent;
- keep zsh migration as onboarding only.

First official packs:

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
- `prompts`;
- `keybindings`.

## Proposed Order

Use [Plugin System Roadmap](plugin-system-roadmap.md) as the active execution
sequence. The current branch has already completed the read-only registry,
`[plugins]` parsing, `winuxsh plugin list/info`, and the first
`oh-my-winuxsh` bundle scaffold. The next step is managed TOML plan/apply for
plugin enable/disable.

## Zsh Migration Boundary

Zsh compatibility remains a migration adapter:

- read `.zshrc` safely;
- detect familiar workflow intent;
- suggest Winuxsh plugins;
- never claim zsh plugin support;
- never execute ZLE, `zmodload`, `zpty`, or arbitrary zsh plugin scripts.

Wording should prefer:

- "zsh migration";
- "official Winuxsh plugins";
- "keybinding presets";
- "Winuxsh plugin bundle".

Avoid:

- "zsh plugin support";
- "native zsh packs";
- "standard ZLE widgets".

## Test Gate

- Keep the v2 baseline green before each v3 change:
  - `cargo fmt --check`
  - `cargo test --lib -p winuxsh-runtime --locked`
  - `cargo test -p winuxsh-runtime --test completion --locked`
  - `cargo build --locked`
  - `cargo test --test compat -- --ignored`
- Add registry and manifest tests before enabling user-facing plugin behavior.
- Add plan/apply/rollback tests before writing user TOML.
