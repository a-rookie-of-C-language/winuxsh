---
tags: [niubash, setup, wizard, ux, nerd-font, presets]
created: 2026-09-14
status: proposed
---

# Setup Wizard Redesign

> Context: unixwin/niubash#85 reported that `niu setup` mishandles input on
> Windows terminals. The input bugs are fixed in
> `crates/niubash-runtime/src/interactive_menu.rs`; this document covers the
> deeper redesign — the wizard asks too many questions, has no presets, and
> does not handle Nerd Fonts, which several bundled themes require.

## Current Wizard Audit

First-run question sequence (up to 12 prompts):

1. Setup preset (`minimal`/`dev`/`full`) — installs wpm packages
2. Proceed with installation? (y/n, text prompt)
3. Enable bundled prompt/theme plugins? (y/n, text prompt)
4. Prompt path display (`home`/`full`/`basename`)
5. Colour theme — flat list of ~27 themes
6. Prompt symbol (5 options)
7. Prompt style (`minimal`/`classic`/`powerline`/`multiline`/`segments`)
8. Segment preset (conditional on `segments`)
9. Right-side info (`off`/`time`/`full`)
10. Show git in prompt? (y/n)
11. Use Starship for git segment? (y/n, conditional)
12. Tab completion style (`column`/`list`/`inline`)

### Problems

- **No preset escape hatch.** Every user answers every question. The rc the
  wizard writes (`NIU_PLUGINS=(prompt-core git)`) is far below what a
  configured shell looks like — a real setup uses a dozen packs plus aliases.
- **Package install is mixed into shell config.** wpm installs run before any
  config question, mid-flow, and print `(wpm not available)` noise when wpm
  is missing.
- **Two interaction models.** Choices use the arrow-key menu; y/n questions
  use `read_line` text prompts. Pick one.
- **27-item flat theme menu.** The preview block prints once before the menu
  and does not track the highlight; users cannot see what they are choosing.
- **No Nerd Font handling.** Themes tagged `[Nerd Font]` (agnoster, p10-*,
  spaceship, tokyonight, …) render boxes on stock fonts. The wizard neither
  detects nor resolves this.
- **Environment-blind defaults.** git/starship/fzf presence, terminal type
  (`WT_SESSION`, MinTTY, conhost), and installed fonts are never probed, so
  defaults cannot adapt.
- **Silent MinTTY fallback.** Under Git Bash stdin is a pipe,
  `stdio_is_interactive()` is false, and every question silently takes its
  default with no message.
- **Odd Esc semantics.** Esc means "silently use this question's default".
  Users expect cancel/quit, or at minimum "defaults for everything left".
- **No summary/confirmation** before `~/.niubashrc` is written.
- **Existing integrations unused.** `--install-wt-profile` already upserts a
  Windows Terminal profile, but the wizard never offers it.

## Reference Practices

| Project | Approach | Takeaway |
| --- | --- | --- |
| p10k `configure` | Preflight checks (TTY, size ≥47×14, writable rc); **glyph tests** ("does this look like a diamond?") instead of font APIs; offers font install on supported terminals; live previews per question; generates a heavily commented config | Capability-test the glyphs, handle font before style, preview what is selected |
| oh-my-posh | `font install <name>` downloads a Nerd Font zip and installs **per-user** (no admin); terminal font config is still manual | Per-user font install is the norm; we can beat it by also writing WT settings |
| starship | `starship preset <name>` applies a whole curated config non-interactively | Presets are data files and a first-class command |
| oh-my-zsh | Installer asks one question; everything else is a commented template | Ask less; produce editable output |
| fish `fish_config` | Browser UI with visual theme/prompt pickers | Out of scope, but reinforces the value of previews |

## Design

### Flow

```text
niu setup
├─ 0. Preflight (no questions)
│     probe terminal (WT_SESSION / conhost / MinTTY), size, Nerd Font
│     presence, winuxcmd command links, and tool availability
│     (git, fzf, eza, starship, wpm, …) → print a short detection summary
│     box; missing command links get a fix step before any question
├─ 1. Font step (conditional)
│     a. NF already installed → skip
│     b. Windows Terminal, no NF → offer install:
│        menu: JetBrainsMono NF (recommended) / MesloLGM NF /
│              CaskaydiaCove NF / skip
│        → download, per-user install, register, set font.face on the
│          Niubash WT profile
│     c. Other terminal → p10k-style glyph test:
│        print sample glyphs, ask "displayed correctly?" (arrow menu)
│        result feeds `nf_capable` for theme filtering
├─ 2. Preset menu — most users finish here
│     recommended / poweruser / minimal / custom
│     (custom → the existing granular questions, all as arrow menus)
├─ 3. Companion tools (first run only, if wpm present)
│     missing recommended tools (eza/fd/rg/fzf/bat/zoxide/dust/duf, plus
│     starship when opted in) are written to `~/.niubash/setup-tools.txt`
│     and installed in one shot via `wpm restore` — the manifest stays on
│     disk so the setup is reproducible
├─ 4. Summary table of every choice → "Apply?" menu
└─ 5. Write ~/.niubashrc (backup first), .setup-done marker, optional
      `--install-wt-profile` offer, closing hints
```

Non-interactive surfaces stay deterministic: when stdio is not interactive the
wizard keeps taking defaults silently (script contract), except MinTTY-style
terminals where stdin is a pipe but a human is present — detectable via
`stdout` being a pipe-less MSYS pty; at minimum print a one-line note that the
menu is unavailable and defaults were used.

### Interaction model

- All questions become arrow-key menus (`interactive_choice`), including y/n.
- `Enter` confirms, digits jump, `Esc` means **"use defaults for all remaining
  questions"** (fast-forward) — surfaced in the help line, not silent.
- Menu highlight optionally re-renders a preview line under the menu so theme
  selection shows the highlighted theme, not a static dump.

### Presets as data

Presets are TOML files shipped in the oh-my-niu bundle under `presets/`
(bundle-owned, versioned with the bundle, extendable by third-party bundles).
The wizard reads them; `niu setup --preset <name>` applies one without the
wizard; `niu plugin presets` (name TBD) can list them.

```toml
# presets/recommended.toml
name = "recommended"
summary = "Curated daily-driver setup: spaceship theme, git, fzf, zoxide, aliases."
requires_nerd_font = true

theme = "spaceship"
prompt_symbol = "❯"
prompt_style = "minimal"
cwd_style = "home"
right_prompt = "time"
completion_style = "column"

packs = [
  "prompt-core", "git", "keybindings", "common-aliases",
  "fzf", "zoxide", "command-not-found", "last-working-dir",
  "themes",
]

# Pack enabled only when the binary exists on PATH.
[conditional_packs]
starship = "starship"
fzf = "fzf"
zoxide = "zoxide"
direnv = "direnv"
kubectl = "kubectl"
docker = "docker"
npm = "npm"

# Aliases written into ~/.niubashrc only when the tool exists.
[conditional_aliases.eza]
ls  = "eza --icons --git --group-directories-first"
ll  = "eza -lh --icons --git --group-directories-first"
la  = "eza -la --icons --git --group-directories-first"
lt  = "eza --tree --level=2 --icons"

[conditional_aliases."bat"]
cat = "bat -pp"
```

Pack names resolve against the bundle's `index.toml` `available` list;
missing packs are skipped with a note, never fatal.

Preset catalog (first cut, distilled from a real configured `~/.niubashrc` —
theme `spaceship`, ~14 packs, eza/bat/dust/duf alias family):

| Preset | Contents |
| --- | --- |
| `recommended` | spaceship theme, minimal prompt, git + keybindings + common-aliases + command-not-found + last-working-dir; conditional fzf/zoxide/starship; eza/bat aliases when present |
| `poweruser` | recommended + direnv, dotenv, thefuck, kubectl/docker/npm (conditional), starship git segment on |
| `minimal` | classic theme (no NF), prompt-core + git, column completion; safe everywhere |
| `custom` | no preset; run the granular flow |

### Nerd Font pipeline

Detection (in order, first hit wins):

1. `WT_SESSION` set → read the Niubash WT profile's `font.face`; if it names a
   `*Nerd Font*`/`*NF`/`MesloLGS` family, treat as capable.
2. Registry enumeration of `HKCU` and `HKLM`
   `SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts` for entries whose name
   contains `Nerd Font` or ends in `NF` → font exists on the machine.
3. Glyph test fallback for non-WT terminals: print ``, ``,
   `` and ask whether they render — the p10k approach that works
   everywhere detection fails.

Install (Windows 10 1809+, no admin):

1. Download the font zip from the nerd-fonts GitHub release
   (`…/releases/latest/download/<Family>.zip`) via the WinHTTP machinery used
   by `self_update.rs`.
2. Extract `*.ttf` (skip `*Windows Compatible*` duplicates and `*.otf`) with
   the existing `zip` dependency into
   `%LOCALAPPDATA%\Microsoft\Windows\Fonts\`.
3. Register each file under
   `HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts`
   (`"<Family> (TrueType)"` → full path) — needs
   `windows-sys` feature `Win32_System_Registry`.
4. `AddFontResourceW` + `SendNotifyMessageW(HWND_BROADCAST, WM_FONTCHANGE)` so
   the font is usable this session — needs `Win32_Graphics_Gdi`.
5. If `WT_SESSION`: set `profiles.list[Niubash].font.face` via
   `windows_terminal.rs` (extend the existing profile JSON with a `font`
   object) and print "restart this tab".
6. Non-WT: print the manual font-face instruction for that terminal
   (conhost can additionally write `HKCU\Console` `FaceName` later).

Fonts are OFL-licensed; downloading from upstream releases at setup time
avoids shipping ~50 MB in the installer. Keep the download host pinned to
`github.com/ryanoasis/nerd-fonts` releases and surface failures as a warning
with the manual URL, never a hard error.

A standalone `niu font` subcommand exposes the same install menu outside
the wizard.

## Phasing

**Phase 1 — wizard structure** (implemented):

- Preset-first flow + preset TOML loading + `--preset` flag
- All questions on `interactive_choice`; Esc = defaults-for-remaining
- Summary table + apply confirmation
- Preflight tool/terminal probing feeding defaults and conditional packs
- MinTTY silent-fallback note; wpm step moved to the end and gated on wpm
  presence

**Phase 2 — font pipeline** (implemented, `crates/niubash-runtime/src/fonts.rs`):

- Detection: font-directory scan + `HKCU`/`HKLM` Fonts registry enumeration,
  glyph test as the universal fallback
- `niu font` subcommand: menu → download the nerd-fonts release zip via the
  system `curl.exe` → extract the `*NerdFontMono-*` family TTFs into
  `%LOCALAPPDATA%\Microsoft\Windows\Fonts` → write `HKCU\...\Fonts` values →
  `AddFontResourceW` + `PostMessageW(HWND_BROADCAST, WM_FONTCHANGE)`
- WT `font.face` written through `windows_terminal.rs` — immediately on the
  existing Niubash profile, and folded into the profile when the wizard
  registers one
- `[Nerd Font]` theme gating on the capability result

The wpm index carries no font packages (checked `official.json`), so fonts
download straight from `github.com/ryanoasis/nerd-fonts` release assets.
wpm package names differ from binary names for ripgrep: the setup manifest
lists `ripgrep` while detection and verification probe `rg`.

## Decisions

- **Preset location: bundle `presets/` + built-in fallback.** oh-my-niu is
  preinstalled with the distribution, so the preset catalog lives in the
  bundle where it can be versioned alongside the packs it references. Niubash
  still compiles in the same three presets as a fallback so `niu setup` never
  hard-fails when the bundle is missing; a bundle preset with the same name
  overrides the built-in.
- **Starship has three modes, all opt-in and installable.** The wizard asks
  once: `Built-in` (native snapshot), `Starship segment` (exports
  `NIU_PROMPT_GIT_BACKEND=starship` before the bundle loads so only the
  `{git}` segment is delegated — niubash themes still apply), or
  `Full Starship` (the bundle `starship` plugin runs `starship init bash`
  and owns the whole prompt; the generated rc skips the niubash prompt
  template entirely). `poweruser` defaults to the segment mode. A missing
  binary is fetched through wpm in the tools step.
- **Companion tools install as cumulative bundles.** Instead of per-tool
  questions the wizard offers `Essentials` (eza fd ripgrep fzf bat),
  `Modern CLI` (+ zoxide dust duf delta sd procs), and `Everything`
  (+ atuin bottom lazygit jq yq xh hyperfine tokei glow navi watchexec),
  filtered to what is actually missing. The selection is written to
  `~/.niubash/setup-tools.txt` and installed in one shot via
  `wpm restore`; wpm resolves through `wpm` on PATH or `winuxcmd.exe wpm`.
- **Esc = defaults for remaining questions; Ctrl+C = abort.** Esc
  fast-forwards through the rest of the wizard taking every default, then
  still shows the summary so the user sees what will be written. Ctrl+C aborts
  the wizard without writing anything. The help line says so explicitly.
- **conhost `FaceName`: deferred.** Windows Terminal is the supported target;
  conhost font registration has its own quirks (`TrueTypeFont` console list)
  for little payoff. Non-WT terminals get the glyph test plus manual
  instructions.
