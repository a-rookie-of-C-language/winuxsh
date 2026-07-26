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

Start with process-based plugins before WASM:

- Process plugins are easiest to debug on Windows and preserve current PATH,
  stdio, and environment semantics.
- A manifest can declare commands, completions, hooks, and required external
  tools without extending rubash parser internals.
- WASI/WASM can be evaluated after the host API is stable and narrow.

## Minimum manifest surface

```toml
name = "example"
version = "0.1.0"

[commands]
hello = "example-plugin.exe hello"

[hooks]
precmd = ["example-plugin.exe precmd"]
chpwd = ["example-plugin.exe chpwd"]

[completions]
dirs = ["completions"]
```

## Non-goals

- No arbitrary ZLE widget execution.
- No parser or executor extensions from plugins.
- No automatic trust of remote plugin code.
- No dependency on `winuxcmd`; plugin commands should work with ordinary PATH
  lookup and user-selected coreutils.
