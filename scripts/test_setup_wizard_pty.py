#!/usr/bin/env python3
"""Isolated PTY smoke test for `niu setup`.

Spawns the niu.exe under test inside a ConPTY with HOME/USERPROFILE/
LOCALAPPDATA pointed at a fresh temp dir and PATH limited to the smoke
winuxcmd + System32 — nothing touches the real profile, the installed
binary, or the real font/registry state. Assertions run against the
*rendered* screen (pyte parses the ANSI stream), so they catch in-place
redraw bugs that raw byte matching cannot see.

Requires pywinpty and pyte (both present in the dev image).

Usage:
    python scripts/test_setup_wizard_pty.py [path\\to\\niu.exe]

Default binary: target\\release\\niu.exe. Local-only gate, like the
bash-upstream runner — not part of cargo test or CI.
"""

import os
import re
import sys
import tempfile
import threading
import time
from pathlib import Path

import pyte
from winpty import PtyProcess

REPO = Path(__file__).resolve().parent.parent
SMOKE = Path(os.environ.get("NIU_SMOKE_ROOT", r"K:\niu-smoke"))

UP = "\x1b[A"
DOWN = "\x1b[B"
ENTER = "\r"
ESC = "\x1b"
CTRL_C = "\x03"


class Session:
    """A niu.exe instance under ConPTY with a pyte-parsed screen."""

    def __init__(self, exe: Path, home: Path, cols: int = 120, rows: int = 36):
        winuxcmd = SMOKE / "winuxcmd"
        sysroot = os.environ.get("SystemRoot", r"C:\Windows")
        env = {
            "SystemRoot": sysroot,
            "COMSPEC": sysroot + r"\System32\cmd.exe",
            "HOME": str(home),
            "USERPROFILE": str(home),
            "LOCALAPPDATA": str(home / "local-appdata"),
            "NIU_APP_BUNDLE_PATH": str(SMOKE / "bundles" / "oh-my-niu"),
            "PATH": os.pathsep.join(
                [
                    str(winuxcmd / "bin"),
                    str(winuxcmd / "usr" / "bin"),
                    sysroot + r"\System32",
                    sysroot,
                ]
            ),
        }
        self.proc = PtyProcess.spawn(
            [str(exe)], cwd=str(home), env=env, dimensions=(rows, cols)
        )
        self.screen = pyte.Screen(cols, rows)
        self.stream = pyte.Stream(self.screen)
        self._reader = threading.Thread(target=self._pump, daemon=True)
        self._reader.start()

    def _pump(self):
        while self.proc.isalive():
            try:
                data = self.proc.read()
            except Exception:
                break
            if data:
                self.stream.feed(data)

    def text(self) -> str:
        return "\n".join(self.screen.display)

    def send(self, keys: str, settle: float = 0.6):
        self.proc.write(keys)
        self.wait_idle(settle)

    def wait_idle(self, quiet: float = 0.5, timeout: float = 20):
        """Return once the rendered screen stops changing."""
        deadline = time.time() + timeout
        last = None
        while time.time() < deadline:
            cur = list(self.screen.display)
            if cur == last:
                return True
            last = cur
            time.sleep(max(0.05, quiet / 5))
        return False

    def wait_for(self, *needles: str, timeout: float = 30) -> str:
        deadline = time.time() + timeout
        while time.time() < deadline:
            body = self.text()
            for n in needles:
                if n in body:
                    return n
            time.sleep(0.1)
        raise TimeoutError(
            f"timed out waiting for {needles}; screen:\n{body}"
        )

    def highlighted(self) -> list[str]:
        # Answered menus keep their ◆ row on screen — the active menu's
        # highlight is always the last one.
        return [ln.strip() for ln in self.screen.display if "◆" in ln]

    def active_highlight(self) -> str:
        return self.highlighted()[-1]

    def option_number(self, label: str, after: str = "") -> str | None:
        """Number of `label` in the active menu — searches only the text
        after the last occurrence of `after` so scrolled-off menus cannot
        produce a stale match."""
        body = self.text()
        seg = body[body.rfind(after):] if after else body
        m = re.search(rf"(\d+)\) {re.escape(label)}", seg)
        return m.group(1) if m else None

    def active_label(self, needles: list[str]) -> str | None:
        """The bottommost needle on screen — the currently active menu.
        Answered menus stay visible above, so position, not membership,
        decides what is live."""
        body = self.text()
        best, pos = None, -1
        for n in needles:
            p = body.rfind(n)
            if p > pos:
                pos, best = p, n
        return best

    def close(self):
        try:
            self.proc.terminate(force=True)
        except Exception:
            pass


def fresh_home() -> tempfile.TemporaryDirectory:
    return tempfile.TemporaryDirectory(prefix="niu-wizard-test-")


def check(cond: bool, msg: str):
    if not cond:
        raise AssertionError(msg)


def reach_preset_menu(s: Session):
    """Answer any menus that precede the preset question (font offer, link
    repair) with the safe offline choice, until the preset menu is live."""
    handled: set[str] = set()
    deadline = time.time() + 60
    while time.time() < deadline:
        which = s.active_label(["preset applies", "keep my current font", "glyphs above"])
        if which == "preset applies":
            return
        if which is None or which in handled:
            time.sleep(0.3)
            continue
        handled.add(which)
        if which == "keep my current font":
            s.send("4")  # Skip — the test stays offline
            s.send(ENTER)
        else:
            s.send(ENTER)  # glyph test → No (default)
    raise TimeoutError("preset menu never became active:\n" + s.text())


def test_single_step_and_no_duplication(exe: Path):
    """One Down moves the highlight exactly one row; nothing duplicates."""
    with fresh_home() as home:
        s = Session(exe, Path(home))
        try:
            reach_preset_menu(s)
            before = s.active_highlight()
            check("recommended" in before, f"default should be recommended: {before}")

            s.send(DOWN)
            after = s.active_highlight()
            check("poweruser" in after, f"after Down: expected poweruser, got {after}")
            check(
                s.text().count("recommended curated") == 1,
                "option row duplicated on screen:\n" + s.text(),
            )

            s.send(UP)
            check(
                "recommended" in s.active_highlight(),
                f"after Up: expected recommended again, got {s.active_highlight()}",
            )
        finally:
            s.close()


def test_esc_fast_forwards_to_summary(exe: Path):
    """Esc on the first question means defaults for everything remaining."""
    with fresh_home() as home:
        s = Session(exe, Path(home))
        try:
            reach_preset_menu(s)
            s.send(ESC)
            s.wait_for("Apply this configuration")
            s.send(CTRL_C)
            s.wait_for("nothing was written")
            check(
                not (Path(home) / ".niubashrc").exists(),
                "Ctrl+C abort must not write ~/.niubashrc",
            )
        finally:
            s.close()


def test_long_menu_windows_and_redraws(exe: Path):
    """The 27-theme menu must window inside the viewport and redraw cleanly."""
    with fresh_home() as home:
        s = Session(exe, Path(home))
        try:
            reach_preset_menu(s)
            s.send("4")  # custom
            s.send(ENTER)
            s.wait_for("prompt/theme plugins")
            s.send(ENTER)  # Yes
            s.wait_for("path display")
            s.send(ENTER)  # home
            s.wait_for("Colour theme")

            body = s.text()
            check(
                "more" in body,
                "long theme list should show a scroll indicator:\n" + body,
            )
            for _ in range(3):
                s.send(DOWN)
            check(
                len(s.highlighted()) >= 1,
                f"expected a highlight, got {s.highlighted()}",
            )
            s.send(ESC)
            s.wait_for("Apply this configuration")
            s.send(CTRL_C)
            s.wait_for("nothing was written")
        finally:
            s.close()


def test_full_apply_writes_rc(exe: Path):
    """Accept the recommended preset end-to-end; rc lands only in the sandbox."""
    labels = [
        "keep my current font",
        "glyphs above",
        "preset applies",
        "Git prompt engine",
        "Install starship via wpm",
        "companion tools via wpm",
        "Windows Terminal",
        "Apply this configuration",
    ]
    with fresh_home() as home:
        s = Session(exe, Path(home))
        try:
            handled: set[str] = set()
            deadline = time.time() + 120
            while time.time() < deadline:
                body = s.text()
                if "You can tweak" in body or "Nothing was written" in body:
                    break
                which = s.active_label(labels)
                if which is None or which in handled:
                    time.sleep(0.3)
                    continue
                handled.add(which)
                if which == "keep my current font":
                    num = s.option_number("Skip", after="keep my current font")
                    if num:
                        s.send(num)
                    s.send(ENTER)
                elif which == "glyphs above":
                    s.send(ENTER)  # No (default)
                elif which == "preset applies":
                    s.send(ENTER)  # recommended
                elif which == "Git prompt engine":
                    s.send(ENTER)  # Built-in
                elif which == "Install starship via wpm":
                    s.send(DOWN)
                    s.send(ENTER)  # No — keep the test offline
                elif which == "companion tools via wpm":
                    num = s.option_number("Skip", after="companion tools")
                    if num:
                        s.send(num)
                    s.send(ENTER)
                elif which == "Windows Terminal":
                    s.send(DOWN)
                    s.send(ENTER)  # No
                elif which == "Apply this configuration":
                    s.send(ENTER)  # Apply is the default highlight
            else:
                raise TimeoutError("wizard never finished:\n" + s.text())
            check(
                (Path(home) / ".niubashrc").exists(),
                "Apply should write ~/.niubashrc inside the sandbox\n" + s.text(),
            )
            real_rc = Path.home() / ".niubashrc"
            check(
                not real_rc.exists()
                or real_rc.stat().st_mtime < time.time() - 120,
                "real profile rc must not be touched by the test",
            )
            # First interactive prompt must not spam hook errors.
            s.send(CTRL_C)
            s.wait_idle(1.5)
            check(
                "command not found" not in s.text(),
                "hook registry errors on prompt:\n" + s.text(),
            )
        finally:
            s.close()


def main() -> int:
    exe = Path(sys.argv[1]) if len(sys.argv) > 1 else REPO / "target" / "release" / "niu.exe"
    if not exe.is_file():
        print(f"niu.exe not found: {exe}")
        return 2

    tests = [
        test_single_step_and_no_duplication,
        test_esc_fast_forwards_to_summary,
        test_long_menu_windows_and_redraws,
        test_full_apply_writes_rc,
    ]
    failed = 0
    for t in tests:
        try:
            t(exe)
            print(f"PASS {t.__name__}")
        except Exception as err:
            failed += 1
            print(f"FAIL {t.__name__}: {err}")
    print(f"\n{len(tests) - failed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
