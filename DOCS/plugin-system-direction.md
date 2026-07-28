# Plugin system direction

This note tracks issue #22: winuxsh should not promise full zsh compatibility.
Zsh compatibility is a migration layer for aliases, simple shell snippets,
completion assets, and a few native UX presets. It is not a ZLE, zmodload, or
zsh interpreter clone.

## Decision

- Keep zsh import/report support as a compatibility adapter.
- Keep native plugin presets explicit through `[zsh.native_widgets]` and
  `[zsh.native_plugins]`.
- Do not source arbitrary zsh plugin scripts during normal startup.
- Define an `oh-my-winuxsh` package layer around native manifests, not around
  vendored Oh My Zsh source.

## Runtime shape

Use Winuxsh-native WASM/WASI as the target plugin model:

- WASM plugins get a narrow, versioned host API for completion providers,
  prompt segments, lifecycle hooks, and helper commands.
- The host API must make capabilities explicit: filesystem, environment, PATH,
  network, process execution, and terminal interaction are opt-in.
- Process plugins can remain a debugging bridge or compatibility adapter for
  existing Windows-native tools, but they are not the long-term package model.
- No plugin model may extend rubash parser or executor internals.

## Minimum manifest surface

```toml
name = "example"
version = "0.1.0"
kind = "wasm"
module = "example-plugin.wasm"
permissions = ["cwd:read", "env:read"]

[commands]
hello = "plugin example hello"

[hooks]
precmd = ["plugin example precmd"]
chpwd = ["plugin example chpwd"]

[completions]
dirs = ["completions"]
```

## Non-goals

- No arbitrary ZLE widget execution.
- No parser or executor extensions from plugins.
- No automatic trust of remote plugin code.
- No dependency on `winuxcmd`; plugin commands should work with ordinary PATH
  lookup and user-selected coreutils.
- No direct sourcing of zsh plugin scripts as a substitute for a native plugin.
