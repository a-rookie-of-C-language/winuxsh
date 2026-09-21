# Prompt theming and oh-my-niu plugins

Reference for configuring the interactive prompt and selecting/enable-ing
oh-my-niu packs. User-facing entry points: `niu setup` (interactive wizard),
`niu font` (Nerd Font install), `niu doctor` (verify). This document is for
when an AI needs to write or repair `~/.niubashrc` directly.

## The rc contract

```sh
# ~/.niubashrc
NIUBASH="C:/Users/me/.niubash/oh-my-niu"   # installed bundle root
NIU_PLUGINS=(prompt-core git fzf zoxide)   # packs to load, order matters
NIU_THEME=minimal                          # a theme name from themes/
source "$NIUBASH/oh-my-niu.niu"
```

- Sourcing again in the same shell is a no-op; force reload with
  `source "$NIUBASH/oh-my-niu.niu" --reload`.
- User-written plugins live in `~/.niubash/custom/plugins/<name>/`
  (`<name>.plugin.niu` + `plugin.toml`). Never edit files inside the
  installed bundle directory.
- Managed TOML (via `niu env use`) maps to the same system:
  `[plugins] load = [...]` and `[theme] current_theme = "..."`.

## Theme selection (themes/, 28 first-party)

| Want | Theme |
|---|---|
| Powerlevel10k look (needs Nerd Font) | `p10-rainbow`, `p10-classic`, `p10-lean`, `p10-pure` |
| Popular editor palettes | `catppuccin-mocha`, `dracula`, `tokyonight`, `gruvbox`-style: check `themes/` |
| Minimal / no icons | `minimal`, `pure`, `clean`, `classic` |
| Classic oh-my-zsh feel | `robbyrussell`, `agnoster`, `bira`, `fishy`, `spaceship` |
| Light terminal | `light` |
| Everything else | `dark`, `default`, `colorful`, `compact`, `forest`, `ocean`, `lambda` |

Rules of thumb:

- Icon themes (p10-*, agnoster, spaceship) render placeholders without a
  Nerd Font — run `niu font` first or pick a minimal theme.
- Theme is just `NIU_THEME=<name>`; a theme with no matching
  `theme-<name>` plugin falls back to the compiled default.
- Custom themes: `~/.niubash/custom/plugins/theme-<name>/theme-<name>.plugin.niu`,
  then `NIU_THEME=<name>`.

## Pack catalog (plugins/, 21 packs)

| Pack | Gives you | Requires |
|---|---|---|
| `prompt-core` | native prompt segments framework | — |
| `git` | git aliases, completions, prompt segment | `git` |
| `fzf` | fzf keybindings and helpers | `fzf` |
| `zoxide` | `z` smart cd | `zoxide` |
| `starship` | starship prompt bridge | `starship` |
| `docker` | docker aliases/completions | `docker` |
| `kubectl` | kubernetes aliases/completions | `kubectl` |
| `npm` | node aliases, nvm-style helpers | `node` |
| `thefuck` | correction hook | `thefuck` |
| `dotenv` | auto-load `.env` per directory | — |
| `auto-env` | auto node/python env activation | — |
| `env-sync` | sync env packs with managed TOML | — |
| `dirmarks` | directory bookmarks (`mark`/`jump`) | — |
| `command-not-found` | suggests the wpm package on miss | `wpm` |
| `command-timer` | per-command duration in prompt | — |
| `keybindings` | default keybinding set | — |
| `extract` | universal `extract <archive>` | — |
| `common-aliases` | curated alias set | — |
| `git-fetch-reminder` | nudges stale fetches | `git` |
| `winuxcmd-core` | WinuxCmd command integration segments | — |
| `prompts`, `themes` | segment metadata and theme plugins | — |

Pack selection guidance:

- Keep `NIU_PLUGINS` small; every pack adds rc load time. Start with
  `prompt-core git fzf zoxide command-not-found` and add on demand.
- Packs with `required_binaries` no-op gracefully when the binary is
  missing, but `command -v <bin>` first is still the honest check.
- Enable one alias-heavy pack at a time (`common-aliases`) when debugging
  "where did this alias come from" reports: `type <name>`.

## Validation flow

```bash
source "$NIUBASH/oh-my-niu.niu" --reload
niu doctor                  # full install health
type z                      # confirm the pack's function landed
printf '%s\n' "$PROMPT"     # inspect the rendered prompt, if exported
```

Symptoms → causes:

- Prompt unchanged after editing rc → the loader is re-entrant; use
  `--reload` or open a new shell.
- Boxes/garbled glyphs → Nerd Font missing (`niu font`) or terminal font
  not set.
- Pack function missing → pack not in `NIU_PLUGINS`, or `required_binaries`
  absent.
- Edits inside the installed bundle vanished after upgrade → wrote to the
  bundle instead of `~/.niubash/custom`.
