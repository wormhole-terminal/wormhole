#!/usr/bin/env python3
"""Run esctest2 inside the terminal this process is attached to and write one JSON line per test.

esctest2 (Thomas Dickey's continuation of George Nachman's esctest) sends escape sequences and reads
the terminal's answers back from the tty. This runner starts it the way its own `esctest.py` does, and
changes only what makes a result about the harness and not about the terminal:

- Known-bug annotations are switched off. They say "xterm (or iTerm2) fails this test, so a pass is an
  error"; a terminal that does better than xterm must not be scored as failing. Every test is judged
  against the behaviour the suite describes ("xterm, but without bugs").
- The screen is read back with DECRQCRA, one cell at a time, as the suite does. What the terminal
  answers for a cell is first probed (a positive sum, as xterm's `checksumExtension` 23/31 and iTerm2
  and WezTerm answer, or DEC's negated one), and "empty" and "blank" cells are counted as one: the
  difference cannot be seen on a screen, and terminals disagree on it.
- A terminal that does not answer DECRQCRA cannot be asked what is on the screen. A test that gets to
  a screen check then ends as `unmeasurable`, at once and not after a timeout, and is left out of that
  terminal's pass count (a test that fails before it gets there is a real failure). The same for CSI 18 t
  (the size of the text area in cells): reset() and many tests ask it; when it goes unanswered the size
  is taken from the tty instead and the 18 t tests fail on their own.
- The reverse-wrap tests are run as xterm patch 383 and later behaves (`--xterm-reverse-wrap`): without the
  argument esctest2 assumes the 380-382 behaviour of mode 45, which no current terminal has.
- Tests that need the clipboard are skipped (they would overwrite it), unless --clipboard is given.

Nothing of esctest2 is edited; it is imported from --esctest. Results go to --out as JSON lines: one
`probe` line, one `test` line per test, one `summary` line.
"""
import argparse
import fcntl
import json
import os
import re
import select
import signal
import struct
import sys
import termios
import time
import traceback
import tty
import types

ESC = "\x1b"
CLIPBOARD_CLASSES = ("ManipulateSelectionDataTests",)
PER_TEST_SECONDS = 30


class NotMeasurable(Exception):
    """The terminal cannot be asked what this test needs to see."""


class TestTimeout(Exception):
    """A test took longer than PER_TEST_SECONDS."""


# ---------------------------------------------------------------- talking to the terminal (probe)

def write(s):
    os.write(1, s.encode("latin-1"))


def read_until(pattern, timeout):
    """Read from the tty until `pattern` (a compiled regex over latin-1 text) matches; returns the
    match or None when `timeout` seconds pass first. Bytes before the match are dropped."""
    buf = ""
    deadline = time.monotonic() + timeout
    while True:
        m = pattern.search(buf)
        if m:
            return m
        left = deadline - time.monotonic()
        if left <= 0:
            return None
        ready, _, _ = select.select([0], [], [], left)
        if ready:
            data = os.read(0, 4096)
            if not data:
                return None
            buf += data.decode("latin-1")


def drain(quiet=0.03):
    """Throw away whatever arrives until the line has been quiet for `quiet` seconds."""
    while True:
        ready, _, _ = select.select([0], [], [], quiet)
        if not ready:
            return
        os.read(0, 4096)


def ask(seq, pattern, timeout):
    drain(0.02)
    t0 = time.monotonic()
    write(seq)
    m = read_until(re.compile(pattern), timeout)
    ms = round((time.monotonic() - t0) * 1000)
    return m, ms


def probe(timeout):
    """What the terminal answers of the reports esctest2 needs. Returns a dict for the result file."""
    p = {}
    m, ms = ask(f"{ESC}[6n", r"\x1b\[(\d+);(\d+)R", timeout)
    p["cpr"] = bool(m)
    p["cpr_ms"] = ms if m else None
    m, _ = ask(f"{ESC}[c", r"\x1b\[\?([\d;]*)c", timeout)
    p["da1"] = m.group(0).replace(ESC, "<ESC>") if m else None
    m, _ = ask(f"{ESC}[>c", r"\x1b\[>([\d;]*)c", timeout)
    p["da2"] = m.group(0).replace(ESC, "<ESC>") if m else None
    m, _ = ask(f"{ESC}[18t", r"\x1b\[8;(\d+);(\d+)t", timeout)
    p["csi_18t"] = [int(m.group(1)), int(m.group(2))] if m else None
    m, _ = ask(f"{ESC}[14t", r"\x1b\[4;(\d+);(\d+)t", timeout)
    p["csi_14t"] = [int(m.group(1)), int(m.group(2))] if m else None
    m, _ = ask(f"{ESC}[16t", r"\x1b\[6;(\d+);(\d+)t", timeout)
    p["csi_16t"] = [int(m.group(1)), int(m.group(2))] if m else None
    try:
        p["tty_size"] = list(struct.unpack("hhhh", fcntl.ioctl(0, termios.TIOCGWINSZ, b"\0" * 8))[:2])
    except OSError:
        p["tty_size"] = None
    # DECRQCRA of one written cell, one erased cell and one never written: the sum for 'a' says
    # whether the terminal answers positively (0x0061) or as DEC does (0xFF9F).
    write(f"{ESC}[2J{ESC}[H" + "a" + f"{ESC}[3;1H" + f"{ESC}[2K")
    pat = r"\x1bP(\d+)!~([0-9a-fA-F]+)\x1b\\"
    sums = {}
    for name, (row, col) in (("a", (1, 1)), ("erased", (3, 1)), ("untouched", (5, 5))):
        m, _ = ask(f"{ESC}[7;0;{row};{col};{row};{col}*y", pat, timeout)
        sums[name] = int(m.group(2), 16) if m else None
    p["decrqcra"] = sums["a"] is not None
    p["decrqcra_sums"] = sums
    if sums["a"] == 0x61:
        p["checksum"] = "positive"
    elif sums["a"] == 0x10000 - 0x61:
        p["checksum"] = "negated"
    else:
        p["checksum"] = "other" if sums["a"] is not None else None
    write(f"{ESC}[2J{ESC}[H")
    drain(0.1)
    return p


# ---------------------------------------------------------------- the suite

def load_esctest(esctest_dir, argv):
    """Import esctest2's modules and return the namespace of its `esctest.py` without running its
    `main()` (the file calls it at the bottom, unconditionally)."""
    sys.path.insert(0, esctest_dir)
    sys.argv = ["esctest.py"] + argv
    with open(os.path.join(esctest_dir, "esctest.py"), encoding="utf-8") as f:
        src = f.read().rstrip()
    tail = "\nmain()"
    if not src.endswith(tail):
        raise SystemExit("esctest.py does not end with main(): the pinned version changed, update this runner")
    return src[: -len("main()")], esctest_dir


def run(args):
    saved = termios.tcgetattr(0)
    tty.setraw(0)
    try:
        return run_raw(args)
    finally:
        termios.tcsetattr(0, termios.TCSADRAIN, saved)


def run_raw(args):
    out = open(args.out, "w", encoding="utf-8", buffering=1)

    def emit(obj):
        out.write(json.dumps(obj, sort_keys=True) + "\n")

    pr = probe(args.timeout)
    emit({"type": "probe", "label": args.label, **pr})
    if args.probe_only:
        return 0

    # The checksum convention follows what the terminal answered; without an answer there is no
    # screen to read and every screen check is unmeasurable.
    mode = args.checksum if args.checksum != "auto" else pr["checksum"]
    can_read_screen = pr["decrqcra"] and mode in ("positive", "negated")

    esctest_argv = [
        "--expected-terminal", "xterm",
        "--xterm-checksum", "334",
        "--xterm-reverse-wrap", str(args.xterm_reverse_wrap),
        "--timeout", str(args.timeout),
        "--max-vt-level", str(args.max_vt_level),
        "--include", args.include,
        "--logfile", args.log,
        "--no-print-logs",
    ]
    src, esctest_dir = load_esctest(args.esctest, esctest_argv)

    # esctest2's modules, in the order its own esctest.py imports them (esccmd and escutil import each other).
    import esc  # noqa: E402
    import escargs  # noqa: E402,F401
    import esccmd  # noqa: E402,F401
    import escutil  # noqa: E402
    import esctypes  # noqa: E402

    # Known-bug annotations off (see the module docstring); the terminals they name are kept as data.
    def known_bug(terminal, reason, noop=False, shouldTry=True):
        def decorator(func):
            func.__dict__.setdefault(escutil.KNOWN_BUG_TERMINALS, {})[terminal] = reason
            return func
        return decorator

    def option_unchanged(terminal, option, allowPassWithoutOption=False):
        def decorator(func):
            return func
        return decorator

    escutil.knownBug = known_bug
    escutil.optionRequired = option_unchanged
    escutil.optionRejects = option_unchanged

    # "Empty" and "blank" cells are one thing here.
    esc.empty = lambda: " "
    esc.blank = lambda: " "

    real_checksum = escutil.GetChecksumOfRect

    def checksum_of_rect(rect):
        if not can_read_screen:
            raise NotMeasurable("DECRQCRA unanswered" if not pr["decrqcra"] else "unknown checksum form")
        raw = real_checksum(rect)
        if mode == "negated":
            raw = (0x10000 - raw) & 0xFFFF
        return raw if raw != 0 else 0x20

    escutil.GetChecksumOfRect = checksum_of_rect

    if pr["csi_18t"] is None and pr["tty_size"]:
        def screen_size():
            rows, cols = struct.unpack("hhhh", fcntl.ioctl(0, termios.TIOCGWINSZ, b"\0" * 8))[:2]
            return esctypes.Size(cols, rows)
        escutil.GetScreenSize = screen_size

    ns = types.ModuleType("esctest_main")
    ns.__file__ = os.path.join(esctest_dir, "esctest.py")
    exec(compile(src, ns.__file__, "exec"), ns.__dict__)
    ns.init()

    def on_alarm(signum, frame):
        raise TestTimeout(f"no end after {PER_TEST_SECONDS}s")

    signal.signal(signal.SIGALRM, on_alarm)

    counts = {}
    started = time.monotonic()
    for name, method in ns.MatchingNamesAndMethods():
        cls, _, fn = name.partition(".")
        rec = {"type": "test", "name": name, "class": cls, "status": None, "reason": None}
        kbt = method.__dict__.get(escutil.KNOWN_BUG_TERMINALS, {})
        rec["xterm_known_bug"] = "xterm" in kbt
        if cls in CLIPBOARD_CLASSES and not args.clipboard:
            rec["status"], rec["reason"] = "skip", "would overwrite the clipboard"
        else:
            drain(0.03)
            t0 = time.monotonic()
            signal.setitimer(signal.ITIMER_REAL, PER_TEST_SECONDS)
            try:
                try:
                    ns.reset()
                except NotMeasurable:
                    raise
                except Exception as e:  # the suite's own set-up failed: nothing was tested
                    rec["status"] = "blocked"
                    rec["reason"] = "reset: " + one_line(e)
                if rec["status"] is None:
                    method()
                    escutil.AssertAssertionAsserted()
                    rec["status"] = "pass"
            except NotMeasurable as e:
                rec["status"], rec["reason"] = "unmeasurable", str(e)
            except esctypes.InsufficientVTLevel as e:
                rec["status"], rec["reason"] = "skip", "vt level: " + one_line(e)
            except TestTimeout as e:
                rec["status"], rec["reason"] = "fail", "hung: " + str(e)
            except esctypes.KnownBug as e:  # cannot happen with the decorators off; kept for safety
                rec["status"], rec["reason"] = "skip", "known bug: " + one_line(e)
            except Exception as e:
                rec["status"] = "fail"
                rec["reason"] = classify(e)
                with open(args.log + ".trace", "a", encoding="utf-8") as tf:
                    tf.write("*** %s\n%s\n" % (name, traceback.format_exc()))
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
            rec["ms"] = round((time.monotonic() - t0) * 1000)
        counts[rec["status"]] = counts.get(rec["status"], 0) + 1
        emit(rec)

    try:
        ns.reset()
    except Exception:
        pass
    emit({"type": "summary", "counts": counts, "seconds": round(time.monotonic() - started, 1)})
    try:
        ns.shutdown()
    except Exception:
        pass
    return 0


def one_line(e):
    text = " ".join(str(e).split())
    return (type(e).__name__ + ": " + text)[:300]


def classify(e):
    """A short reason: a read that timed out is told apart from a wrong answer."""
    text = str(e)
    if isinstance(e, Exception) and "Timeout waiting to read" in text:
        return "no reply"
    return one_line(e)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--esctest", required=True, help="the esctest2 checkout's esctest/ folder")
    ap.add_argument("--out", required=True, help="JSON lines file")
    ap.add_argument("--log", default="/dev/null", help="esctest2's own log (and <log>.trace for failures)")
    ap.add_argument("--label", default="", help="the terminal's name, kept in the probe line")
    ap.add_argument("--timeout", type=float, default=1.0, help="seconds to wait for a report (esctest2's default)")
    ap.add_argument("--include", default=".*", help="regex over Class.test_name")
    ap.add_argument("--max-vt-level", type=int, default=5)
    ap.add_argument("--checksum", choices=("auto", "positive", "negated"), default="auto")
    ap.add_argument("--xterm-reverse-wrap", type=int, default=383,
                    help="the xterm patch whose reverse-wrap behaviour the tests expect (default 383, current xterm: "
                         "mode 45 follows soft wraps, mode 1045 wraps anywhere; 0 asks for the older mode 45 that did, "
                         "which contradicts the suite's own InitialReverseWraparound test)")
    ap.add_argument("--clipboard", action="store_true", help="also run the tests that overwrite the clipboard")
    ap.add_argument("--probe-only", action="store_true", help="only ask which reports the terminal answers")
    args = ap.parse_args()
    if not os.isatty(0) or not os.isatty(1):
        raise SystemExit("esctest_run.py talks to the terminal it runs in: stdin and stdout must be a tty")
    sys.exit(run(args))


if __name__ == "__main__":
    main()
