# Native builtins: uutils port inventory

Status: native implementations are in place for the current replacement set;
run the final compile/smoke pass after concurrent shell/plugin edits settle.

uutils source snapshot inspected locally:
`target/uutils-coreutils-src` at commit `d169a9a`.

## Goal

Replace the weak rubash "external file builtins" with winuxsh-native Rust
implementations adapted from uutils coreutils, while keeping dependencies and
integration under winuxsh control.

The porting rule is:

- Copy semantics and structure from uutils.
- Do not embed uutils' command crates directly.
- Do not depend on `uucore` or uutils' `clap` command framework.
- Keep one independent Rust file per command under
  `crates/winuxsh-runtime/src/native_file_builtins/`.
- Use winuxsh path resolution, diagnostics, shell integration, and existing
  dependency policy.

## Current builtin layers

### winuxsh overlay builtins

These are intercepted by winuxsh before falling through to rubash.

| Command | Current owner | Notes |
| --- | --- | --- |
| `setopt` | winuxsh | zsh compatibility; keep shell-specific |
| `unsetopt` | winuxsh | zsh compatibility; keep shell-specific |
| `source`, `.` | winuxsh/rubash | shell state mutation; do not replace with uutils |
| `pwd` | winuxsh/rubash | shell `PWD` semantics; can borrow uutils options only |
| `cat` | winuxsh native file builtin | stream-first uutils/winuxcmd-style port |
| `chmod` | winuxsh native file builtin | Windows read-only attribute maps write permission |
| `cp` | winuxsh native file builtin | recursive copy/overwrite/update/preserve first pass |
| `kill` | winuxsh native process builtin | align option surface with winuxcmd; owns real process signalling |
| `mkdir` | winuxsh native file builtin | parents/verbose/mode-compatible parser |
| `mkfifo` | winuxsh native file builtin | explicit Windows unsupported FIFO placeholder |
| `rm` | winuxsh native file builtin | uutils-style option parsing and dash operands |
| `rmdir` | winuxsh native file builtin | parents/verbose/non-empty-ignore first pass |
| `touch` | winuxsh native file builtin | filetime-backed timestamp updates |

### rubash shell builtins

These are shell-language builtins and should not be replaced by uutils unless
there is a very explicit reason. They mutate shell state or are Bash semantics,
not external coreutils semantics.

| Group | Commands |
| --- | --- |
| shell state / variables | `alias`, `unalias`, `export`, `readonly`, `declare`, `typeset`, `local`, `unset`, `set`, `shopt`, `enable`, `getopts` |
| control flow | `:`, `true`, `false`, `eval`, `return`, `break`, `continue`, `shift`, `exit`, `logout` |
| shell execution | `cd`, `command`, `exec`, `source`, `.`, `hash`, `type`, `help` |
| jobs / terminal | `jobs`, `disown`, `wait`, `fg`, `bg`, `suspend`, `umask`, `ulimit`, `times`, `caller`, `trap` |
| input / completion | `read`, `mapfile`, `readarray`, `history`, `bind`, `fc`, `complete`, `compgen`, `compopt` |
| arithmetic / shell tests | `let`, `test`, `[` |
| output-like shell builtins | `echo`, `printf`, `pwd` |

`echo`, `printf`, `pwd`, `test`, and `[` look like coreutils, but when invoked
as shell builtins they must keep shell builtin behavior. For example, shell
`echo --help` should print `--help` rather than help text. uutils can still be
used as a reference for edge cases, but these should remain in the shell builtin
layer unless we add an explicit `command echo`/external mode.

### rubash external file builtins

These were the main replacement target. Winuxsh now intercepts them before
rubash's tiny builtins can run.

| Command | Current native file | uutils source | Status |
| --- | --- | --- | --- |
| `cat` | `cat.rs` | `src/uu/cat/src/cat.rs` | implemented |
| `rm` | `rm.rs` | `src/uu/rm/src/rm.rs` | implemented |
| `mkdir` | `mkdir.rs` | `src/uu/mkdir/src/mkdir.rs` | implemented |
| `touch` | `touch.rs` | `src/uu/touch/src/touch.rs` | implemented |
| `rmdir` | `rmdir.rs` | `src/uu/rmdir/src/rmdir.rs` | implemented |
| `mkfifo` | `mkfifo.rs` | `src/uu/mkfifo/src/mkfifo.rs` | implemented compatibility placeholder |
| `chmod` | `chmod.rs` | `src/uu/chmod/src/chmod.rs` | implemented Windows attribute mapping |
| `cp` | `cp.rs` | `src/uu/cp/src/cp.rs` + `copydir.rs` | implemented first pass |

### winuxcmd exe shims

Current shim policy should stay narrow.

| Command | Current handling | Notes |
| --- | --- | --- |
| `grep` | rewritten to `grep.exe` | keep external for now |
| `rm` | should no longer be rewritten | native port should own it |

## uutils dependency replacement map

This is the important part: copy the behavior, not the dependency stack.

| uutils dependency/pattern | winuxsh replacement |
| --- | --- |
| `clap` app definitions | small per-command parser, plus shared helpers only after duplication is proven |
| `uucore::error`, `UResult`, `show_error!` | return `i32`, print GNU-shaped diagnostics to stderr |
| `uucore::translate`, Fluent locale files | English diagnostics matching GNU/uutils shape |
| `uucore::display::Quotable` | local `quote_path`/`quote_arg` helper |
| `uucore::fs` helpers | `std::fs` plus winuxsh shell-path-to-host-path resolution |
| `uucore::parser` helpers | local parser helpers scoped to each command first |
| `uucore::prompt_yes!` | local terminal-aware prompt helper |
| `indicatif` progress | accept flags initially; implement later only if needed |
| `rustix`, `libc`, Unix safe traversal | avoid unless command truly needs Unix-only semantics |
| `windows-sys` | allowed only for concrete Windows API gaps |
| `filetime` | acceptable for `touch` if std cannot set times cleanly |
| `walkdir` | avoid for `rm`; consider for `cp` only if implementation gets messy |

## Per-command port notes

## uutils parity audit

This audit compares the current winuxsh-native command files against the local
uutils snapshot at `target/uutils-coreutils-src`.

| Command | uutils-facing parity | Known intentional gaps / follow-up |
| --- | --- | --- |
| `cat` | Covers `-A`, `-b`, `-e`, `-E`, `-n`, `-s`, `-t`, `-T`, `-u`, `-v`, stdin, `-`, and streaming output. | Unsafe same-file overwrite protection is delegated away because native builtins are skipped when redirections are present. |
| `rm` | Covers `-f`, `-i`, `-I`, `--interactive`, `--one-file-system`, `--preserve-root`, `--no-preserve-root`, `-r/-R`, `-d`, `-v`, `-g`, `--`, dash operands, and `.`/`..` refusal. | `--one-file-system` and progress are accepted but not full uutils traversal/progress semantics. |
| `mkdir` | Covers `-m/--mode`, `-p/--parents`, `-v/--verbose`, `-Z/--context`, multiple dirs, and `--`. | Mode and SELinux context are parsed for compatibility; Windows mode application is not Unix-equivalent. |
| `touch` | Covers `-a`, `-c/--no-create`, `-d/--date`, `-f`, `-m`, `-r/--reference`, `-t`, `--time`, `--no-dereference`, creation, and filetime-backed timestamp updates. | Date parsing is intentionally small (`@seconds`, touch timestamp, and simple ISO-like forms), not uutils' full parser stack. `-h` remains help for winuxcmd compatibility, while uutils uses `-h` for no-dereference. |
| `rmdir` | Covers `--ignore-fail-on-non-empty`, `-p/--parents`, `-v/--verbose`, multiple dirs, and `--`. | Parent removal is bounded to the user-provided operand parents, not the full resolved absolute path. |
| `mkfifo` | Covers `-m/--mode`, `-Z`, `--context`, multiple names, and `--`. | Windows has no filesystem FIFO equivalent; command remains an explicit unsupported-operation placeholder like winuxcmd. |
| `chmod` | Covers `-c`, `-f/--silent/--quiet`, `-v`, `-R`, `--reference`, `--preserve-root`, `--no-preserve-root`, `--`, numeric modes, symbolic write-bit modes, and recursive traversal. | Windows implementation maps write permission to read-only attribute only; it does not implement full Unix ACL/mode semantics, ownership, symlink traversal, or special bits. |
| `cp` | Covers the major uutils option surface: `-a`, `-b/--backup`, `-d`, `-f`, `-i`, `-H`, `-l`, `-L`, `-n`, `-P`, `-p`, `-R/-r`, `-s`, `-S`, `-t`, `-T`, `--strip-trailing-slashes`, `-u/--update`, `-v`, `-x`, `-Z`, `--remove-destination`, `--attributes-only`, `--parents/--parent`, `--sparse`, `--reflink`, `--preserve`, `--no-preserve`, `--copy-contents`, `--context`, `--debug`, `-g/--progress-bar`, and `--`. | Deep uutils semantics remain incomplete: reflink/sparse/SELinux/progress are accepted no-ops, symlink dereference policy is shallow, self-copy protection is not yet explicit, and metadata preservation is limited to mode/timestamps. |
| `kill` | Covers `-l/--list`, `-t/-L/--table`, `-s/-n/--signal`, obsolete `-9`/`-SIGKILL` style signal args, and Windows process signalling. | `kill -l SIGNAL` currently favors winuxcmd-style list behavior over uutils' signal translation. Jobspec/process-group semantics stay in shell/job-control territory. |

### `rm`

uutils files:

- `src/uu/rm/src/rm.rs`
- `src/uu/rm/src/platform/unix.rs` (Unix safe traversal; do not copy for
  Windows-first native implementation yet)

Semantics to preserve:

- `-f`, `--force`
- `-i`, `-I`, `--interactive[=always|once|never]`
- last occurrence wins for `-f` vs interactive options
- `-r`, `-R`, `--recursive`
- `-d`, `--dir`
- `--preserve-root`, `--preserve-root=all`, `--no-preserve-root`
- `--no-preserve-root` must not be abbreviated
- `.` and `..` operands must be refused
- `--` must stop option parsing so `rm -- -p` removes `-p`

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/rm.rs`

### `cat`

uutils files:

- `src/uu/cat/src/cat.rs`
- `src/uu/cat/src/platform/windows.rs`

Porting shape:

- Must be streaming, not read whole files into memory.
- Must preserve binary output where possible.
- Must handle stdin, multiple operands, `-`, and broken pipe gracefully.
- Options to port first: `-n`, `-b`, `-s`, `-E`, `-T`, `-A`, `-v`.
- This is probably the best next command after `rm`, because internal `cat`
  avoids process spawn overhead.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/cat.rs`

### `kill`

Reference points:

- winuxcmd `kill.exe --help`, `-l`, and `-L`
- uutils `src/uu/kill/src/kill.rs` for parser shape only
- rubash `src/builtins/kill.rs` for signal-list compatibility notes

Porting shape:

- This is not a file builtin, but it should still live in the native builtin
  layer because rubash's current builtin only really handles `-l`/`-L`.
- First pass aligns with winuxcmd: `-s`, `--signal`, `-l`, `--list`, `-L`,
  `--table`, `-h`, `--help`, `-V`, `--version`, `-9`, and `-15`.
- Also accept documented GNU-ish spellings such as `-KILL` and `-SIGKILL`;
  current winuxcmd help advertises them even though the installed parser rejects
  at least some of those forms.
- On Windows, `SIGTERM` attempts a console break and falls back to process
  termination; most other non-zero signals map to process termination.

### `mkdir`

uutils file:

- `src/uu/mkdir/src/mkdir.rs`

Porting shape:

- First pass: `-p`, `-v`, `--`, multiple dirs.
- Mode/SELinux/xattr behavior can be accepted and mostly no-op on Windows, but
  diagnostics should be explicit.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/mkdir.rs`

### `touch`

uutils files:

- `src/uu/touch/src/touch.rs`
- `src/uu/touch/src/error.rs`

Porting shape:

- First pass: create missing files, `-c`, `-a`, `-m`, `-r`, `-t`, `-d`.
- Likely needs `filetime`; avoid pulling uutils' `jiff`/`parse_datetime` stack
  unless we decide timestamp parsing must match GNU deeply.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/touch.rs`

### `rmdir`

uutils file:

- `src/uu/rmdir/src/rmdir.rs`

Porting shape:

- Small and good candidate for early migration.
- First pass: `-p`, `--ignore-fail-on-non-empty`, `-v`, `--`.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/rmdir.rs`

### `mkfifo`

uutils file:

- `src/uu/mkfifo/src/mkfifo.rs`

Porting shape:

- Windows named-pipe semantics are not POSIX FIFO semantics.
- Keep as explicit compatibility command; do not pretend full GNU parity unless
  behavior is defined.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/mkfifo.rs`

### `chmod`

uutils file:

- `src/uu/chmod/src/chmod.rs`

Porting shape:

- Do not promise Unix permissions on Windows.
- Useful first pass: parse modes correctly, implement readonly bit mappings
  where sensible, and make unsupported permission classes explicit.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/chmod.rs`

### `cp`

uutils files:

- `src/uu/cp/src/cp.rs`
- `src/uu/cp/src/copydir.rs`
- `src/uu/cp/src/platform/windows.rs`

Porting shape:

- Largest and most dangerous command in this set.
- Should come after `rm/cat/mkdir/touch/rmdir`.
- Requires careful handling of recursive copy, symlinks/junctions, overwrite
  policy, timestamp preservation, and Windows metadata.

Current native file:

- `crates/winuxsh-runtime/src/native_file_builtins/cp.rs`

## Suggested migration order

1. Done: winuxsh native builtins own
   `rm/cat/mkdir/touch/rmdir/mkfifo/chmod/cp`, plus process builtin `kill`.
2. Done: each command has a separate file under `native_file_builtins/`.
3. Pending final verification: run focused native builtin tests and
   `winuxsh.exe -c` smoke tests once concurrent edits settle.

## Definition of done for each command

- One command file under `native_file_builtins/`.
- No `uucore` dependency.
- No command-level `clap` dependency.
- Uses winuxsh path conversion and shell `PWD`.
- Handles `--` and dash-prefixed operands correctly.
- Has focused tests for:
  - option parsing
  - dash-prefixed operands
  - missing operands
  - Windows path forms
  - first useful uutils/GNU parity cases
- Real `winuxsh.exe -c` smoke test after the other agent's edits settle.
