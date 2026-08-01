---
tags: [winuxsh, plugins, legacy, roadmap]
created: 2026-07-19
updated: 2026-07-30
status: superseded
---

# Legacy Native Pack Plan

This document is retained only as a historical pointer. The active direction is
now:

- [Plugin System Direction](plugin-system-direction.md)
- [Oh My Winuxsh Bundle Plan](oh-my-winuxsh-bundle-plan.md)
- [Winuxsh v3 Design Plan](winuxsh-v3-plan.md)

## Superseded Decision

The old plan used zsh and Oh My Zsh naming as the user-facing frame for
first-party packs. That framing is now deprecated.

Current product direction:

- Winuxsh plugins are Winuxsh-native.
- `oh-my-winuxsh` is the official bundled plugin distribution.
- Existing first-party features should move behind a manifest-backed
  `kind = "builtin"` registry before any WASM work.
- WASM/WASI is the long-term runtime for third-party plugins.
- Process plugins are an adapter/debug bridge.
- Zsh support is an onboarding/migration surface only.
- Winuxsh does not support ZLE runtime or arbitrary zsh plugin execution.

## Historical Mapping

The old "native pack" work should be migrated as follows:

| Old frame | New frame |
| --- | --- |
| zsh native pack inventory | `winuxsh plugin list` |
| zsh-lite profile | official plugin profile plan |
| native widget pack | keybinding presets |
| Oh My Zsh-style git aliases | `oh-my-winuxsh/git` |
| lifecycle native plugins | explicit-trust official packs |
| zsh compatibility report | zsh migration report |

Future implementation should update code and CLI in small compatible steps:

1. keep old commands as deprecated aliases;
2. add `[plugins]` as the canonical config surface;
3. keep old `[zsh.*]` reads for compatibility;
4. migrate docs and new examples to official Winuxsh plugin wording.
