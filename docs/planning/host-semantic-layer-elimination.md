---
tags: [niubash, rubash, architecture, host-layer, semantics, boundary]
created: 2026-09-21
status: proposed
---

# Host Semantic-Layer Elimination

> Context: AGENTS.md says shell semantics belong upstream in
> `unixwin/rubash`; niubash must not carry long-term host-side semantic
> workarounds. This document is the audited inventory of every place the
> niubash host still parses or rewrites shell semantics it does not own,
> plus the dependency-ordered plan and the proven verification protocol for
> removing them. Line numbers are as of 2026-09-21 and will drift — the
> function names are the stable anchors (grep them).

## Precedent — the removal template that already worked twice

1. **`rewrite_virtual_root_args` (deleted 2026-09-21).** A host AST pass
   rewrote literal args `/tmp/x`, `/etc/x`, … to `<install-root>\x`,
   overriding rubash's correct `/tmp` → real `%TEMP%` semantics and
   corrupting `printf '%s\n' /tmp/x` data words. Deleted with its 3 call
   sites, 3 helper fns, and 4 tests (~130 lines). Verified by
   **baseline-compare**: GNU upstream gate failed the same 6 suites before
   and after (pre-existing, environmental), workspace tests green, and a
   sandboxed behavior matrix on the K: smoke drive confirmed `/tmp`
   read/write/cd/ls all unify on `%TEMP%`.
2. **`/tmp` unification (2026-09-21).** Three creation sources of
   `<root>/tmp` removed (niubash tree list in shell.rs, winuxcmd
   `ensure_install_layout` in wpm.cpp, contract doc diagram), and
   completion + syntax-highlighting resolvers aligned to rubash's
   `/tmp` → real temp, `/var/tmp` → `<temp>/var/tmp`.

Both removals followed the same rule: **the host may only delete a pass
once rubash provably owns the semantics, and acceptance is measured
behavior, not code review.**

## Inventory (audited 2026-09-21)

### A. Host reads rubash-internal control-marker protocol — most severe

These know about rubash's private wire format (control characters used
between lexer/parser/executor). If rubash changes its marker scheme, these
silently corrupt user data.

| ID | Site | What it does |
|---|---|---|
| A1 | `rewrite_winuxcmd_command_shims_in_stage` (shell.rs ≈3592) | **DELETED 2026-09-22.** The unconditional `\x11` strip was itself a bug — it turned `echo a\*b` into a live glob (`aXb aYb` before; literal `a*b` after). Tokens now keep carriers; host-side readers of `ast.words` decode through `rubash::decode_to_visible_text` (`decoded_words` helper + call sites). |
| ~~A2~~ | `rewrite_parameter_pattern_words` (shell.rs ≈2845) | **DELETED in 4873627** (C1 merge wave) — `word.strip_prefix('\x1d')` gone. |

### B. Host-side AST/token rewriting passes — the issue-#117 family

"Discover a leak, add another guard" pattern living in the host. Sizes are
current; the whitelist alone is hundreds of entries.

| ID | Site | What it does | rubash coverage that should own it |
|---|---|---|---|
| B1 | `normalize_winuxcmd_slash_drive_args` + `WINUXCMD_PATH_COMMANDS` + `winuxcmd_path_translation_mask` (shell.rs ≈3669 / ≈3772 / ≈3817) | Rewrites `/c/...` args to `C:\...` for a whitelisted command set, with per-command argument masks: grep parses `-e/-f/-m/-A/-B/-C`, sed/awk each a set, **find has a mini find-expression parser in the host to guess which args are paths**. | `external_argument_path` in rubash path.rs translates drive-shaped args at the executor boundary with full quoting context and existence gating. **Precheck 2026-09-22: BLOCKED** — rubash translates `/c/...` unconditionally at argv, so `grep -e /c/pat file` would get `C:/pat` as the *pattern* (mask today protects -e/-f and positional pattern slots; reproduced via rubash.exe directly). `key=/c/val` forms (`dd of=/c/x`) also uncovered. Deletion needs an upstream arg-role story first; keep B1 until then. |
| B2 | `normalize_cd_windows_drive_args` (shell.rs ≈3115) | Normalizes `cd`'s tilde/drive/slash-drive args in the host. `cd` is a rubash builtin. | rubash `cd` builtin should normalize its own operands. **Precheck 2026-09-22: BLOCKED** — bare rubash handles `cd /c/Users`, `cd "C:\Users"`, `cd ~`, but NOT bare `cd /d` (errors "No such file"), `cd /c` (lands `C:/c` not `C:/`), or `cd "C:"` (lands `C:/c`). Upstream the drive-spec operands first. |
| B3 | `normalize_bare_windows_drive_commands` (shell.rs ≈3150) | `C:` as a bare command → rewritten to `cd C:/` by AST surgery (rebuilds `words`, `word_kinds`, `word_metadata`). | A dialect decision that belongs in rubash's command resolution. |
| ~~B4~~ | `normalize_parameter_pattern_operator_order` + `rewrite_parameter_pattern_words` (shell.rs ≈2903 / ≈2829) | **DELETED in 4873627** (C1 merge wave) — engine owns `${var#pat}` operator order natively. | rubash parameter expansion (done). |
| B5 | `rewrite_winuxcmd_command_shims` family (shell.rs ≈3486) | Token-level command-name → shim-path rewriting (duplicates rubash command lookup) + injects `--color=always` into terminal-bound grep stages by token splicing. | Shim routing: rubash command lookup. Grep color: an rc default alias. |

### C. Host text scanners duplicating lexer semantics — medium

| ID | Site | What it does |
|---|---|---|
| C1 | `print_locale_strings` (main.rs ≈273) | Hand-rolled `$"..."` byte scanner for `--dump-strings`/`--dump-po-strings`; misfires inside comments/heredocs. Use the rubash tokenizer instead. |
| C2 | `pretty_print_script` (main.rs ≈308) | Rebuilds source from the token stream because rubash has no AST→source serializer. Right fix: add the serializer API upstream. |
| C3 | `self_update_command_args` (repl.rs ≈83) | String-prefix matching to intercept `self-update` lines before shell parse; aliases/quoting evade it. Minor; acceptable short-term. |

### In-bounds — do not "fix" these

- `script_arg_to_host_path` (main.rs): the bin entry translating **its own
  argv** (`/d/foo` → `D:/foo`) before a Shell exists — process-boundary
  duty, host's job.
- `easter_egg_exit`: routes on rubash's already-parsed AST; parses nothing
  itself.
- `generate_rc` (setup_wizard): the host *writing* a config file is
  config-as-code by design.
- doctor / wizard / interactive menus / completion data tables:
  presentation and data, not semantics.

## Why the order matters — the dependency

**B1 cannot be deleted before A1/A2 are fixed upstream.** The `\x11` strip
(A1) exists precisely because the host rewrites tokens *before* rubash
decodes quoted literals at execution; remove the strip without an upstream
decode-at-boundary guarantee and quoted `/c/...` args reach winuxcmd with
raw `\x11` bytes. The correct order:

1. **Phase 0 (rubash, upstream): decode-at-boundary.** Rubash guarantees
   (or exposes an API for) fully-decoded literal words at the external
   command boundary. Then delete A1 and A2 in niubash. **DONE 2026-09-22:**
   `rubash::decode_to_visible_text` landed (rubash 3de4e165); A1 strip
   deleted, host word-readers decode through `decoded_words`; A2 was
   already gone via 4873627.
2. **Phase 1: delete B1** (mask + whitelist + the find mini-parser,
   ~300+ lines) — rubash `external_argument_path` takes over. **BLOCKED
   2026-09-22:** precheck found rubash translates `/c/...` unconditionally
   at argv, including grep/sed/awk pattern positions (`grep -e /c/pat`
   → `C:/pat`) and misses `key=/c/val` forms. Needs an upstream arg-role
   mechanism; re-run the sandbox matrix after that lands.
3. **Phase 2: upstream B2 and B4** into rubash (cd operand normalization;
   parameter-expansion operator order), then delete host passes. B4 done
   in 4873627; B2 blocked on rubash cd not handling bare `/x` or `X:`
   drive specs (2026-09-22 precheck).
4. **Phase 3: B3 and B5.** B3 is a dialect decision — move it into rubash
   command resolution as designed behavior. B5: let rubash's lookup own
   shim routing; replace grep color injection with a default rc alias.
5. **Phase 4: C1/C2.** C1 is a small PR (tokeniser-based extraction).
   C2 needs a rubash AST→source API first.

Related upstream issue for the fast-path principle:
`unixwin/rubash#117` (text-layer fast paths must converge, GNU-source
comparison required).

## Verification protocol (as proven on 2026-09-21)

1. `cargo fmt --check -p niubash; cargo build --locked;
   cargo test --workspace --locked` — **418 passed** as of writing.
2. `sh scripts/check-rename-clean.sh` — must pass.
3. **GNU upstream gate, baseline-compare**:
   `BASH_RUNNER="C:/Progra~1/Git/bin/bash.exe" bash
   scripts/run-bash-upstream-with-niubash.sh`.
   This workstation has **6 pre-existing environmental failures**
   (`run-appendop`, `run-herestr`, `run-invert`, `run-test`,
   `run-tilde`, `run-tilde2`; 80/86). The acceptance bar is **identical
   failure set before/after the change** — establish the baseline FIRST
   (the `rewrite_virtual_root_args` removal proved these six are not
   caused by host-pass deletions).
4. **Sandboxed behavior matrix on the K: smoke drive**
   (`K:\niubash-smoke`, HOME/USERPROFILE redirected to a sandbox dir —
   never the real home):
   ```bash
   niu -c "printf hi > /tmp/f; cat /tmp/f; cd /tmp && pwd"
   niu -c "printf '%s\n' /tmp/f"          # must print /tmp/f verbatim
   niu -c "ls /c/Users | head -3"          # slash-drive input dialect
   niu -c 'x=/tmp/a; printf "%s\n" "$x"'   # expansions never rewritten
   niu -c 'exit 42'; echo $?               # 42
   ```
### C1 revised scope (audited 2026-09-21, second look)

C1 is NOT a small standalone PR. Findings from the first attempt:
`TokenKind` has no LocaleString variant ($"..." lexes as a plain Word);
`token.value` carries sentinel bytes (`\"`->, `\$`->, escaped-glob
prefix ); the sentinel->visible-text restorer
(`bad_substitution_display`, rubash expand_word.rs:412) is
`pub(in crate::executor)` — not public. So C1 merges into **Phase 0's
API ask**: rubash must expose a public decode-to-visible-text function
(for dump-strings, and for hosts generally). GNU spec anchors for the
output format: third_party/bash/locale.c:550 dump_translatable_strings,
shell.c:507-509, parse.y locale_string. An empty placeholder branch
`fix/c1-tokenizer-dump-strings` was created at 4873627 and deleted.

Phase 0 API landed upstream (rubash master): `rubash::decode_to_visible_text(&str)
-> String` — src/locale.rs, re-exported at crate root. Single-pass carrier
decoder covering CTLESC, the C0 data carriers, word-level prefix markers
(\x1b/\x1c/\x1d), ANSI-C PUA markers, the assignment DATA_* sentinels,
raw-byte marker pairs, and conditional-pattern byte-chars. Hosts decode
`token.value`/`token.raw` through it instead of hand-stripping markers;
this is the prerequisite for deleting A1/A2.

Registered C1 leftovers (not covered by the tokenizer merge):

- **c16 — unclosed `$"` needs an EOF report.** GNU parse.y reports
  unterminated quoted strings at EOF; the extraction path must surface the
  same diagnostic rather than silently ending the string.
- **c20 — nested rewriting inside `$"..."` needs a body serializer.** When
  the string body contains `$(...)`/`${...}`, dump output must reproduce the
  nested expansion verbatim — that requires the rubash AST→source
  serializer (same prerequisite as C2 `pretty_print_script`).

5. rubash's own bridge harness
   (`tests/gnu-compat/run-83.sh check`, ledger
   `docs/COMPATIBILITY-STATUS.md` — 55 zero-diff / 799 raw diff lines as
   of 2026-09-22, after a seed-contamination recount; ~204 of those are
   harness 40s-timeout truncations on `jobs`/`history`) is the semantic
   reference; a brush run through the same
   harness lives in `target/issue-suites/results-brush/` (10/83) as a
   tooling example.

## Cross-repo state the next agent must know

- `D:/repo/rubash` carries one **uncommitted** change:
  `src/bin/bash.rs` (shim prefers `NIU_SHELL`/`niu.exe`, with the
  pre-rename executable names kept as legacy fallbacks). It needs the
  normal upstream PR flow.
- `D:/repo/unixwin-winuxcmd` carries the uncommitted `wpm.cpp` tmp-removal
  plus other in-progress work (grep.cpp etc.) that is **someone else's** —
  do not mix them into one commit.
- The niubash repo itself is staged-but-uncommitted as a whole
  (branch `fix/win10-setup-wizard-keys-logo`, no commits yet).
- K: smoke drive redeploy:
  `cargo build --release --locked && cp target/release/niu.exe
  K:/niubash-smoke/niu.exe` (winuxcmd.exe there is a locally built 1.0.9).

## Product decisions still pending (adjacent, not blocking)

- bash/sh shim exposure as a setup-wizard opt-in (agents keep their bash
  instincts; default-off to avoid shadowing existing Git Bash installs).
- `niu skill install` shipping `skills/niubash/` into agent skill dirs
  (`~/.claude/skills/niubash/...`), plus a doctor row and a wizard
  question.
