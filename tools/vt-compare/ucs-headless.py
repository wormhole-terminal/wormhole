#!/usr/bin/env python3
"""ucs-detect against Wormhole's engine with no window: a pseudo-terminal whose other end is the
`ucs-host` binary (wormhole-vt-bench), which feeds the engine and writes its replies back.

    tools/vt-compare/ucs-headless.py [--out result.yaml] [--host path/to/ucs-host] [-- ucs-detect args]

Runs ucs-detect's default measurement (the same one scripts/suite-compare.sh runs inside each app)
and prints the share of sequences sized as wcwidth says, per category. It needs the cache that
fetch-suites.sh makes. It starts no app and opens no window.
"""
import argparse
import fcntl
import os
import pty
import struct
import subprocess
import sys
import termios
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CATEGORIES = [
    ("wide", "unicode_wide_results"), ("narrow", "narrow_results"), ("ZWJ", "emoji_zwj_results"),
    ("VS16", "emoji_vs16_results"), ("VS15", "emoji_vs15_results"), ("flags", "ri_results"),
    ("lone RI", "sri_results"), ("lone skin tone", "sfz_results"),
    ("languages", "language_results"),
]


def suites_env():
    out = subprocess.run([os.path.join(HERE, "fetch-suites.sh"), "--env"], check=True,
                         capture_output=True, text=True).stdout
    return dict(line.split("=", 1) for line in out.splitlines() if "=" in line)


def pump(src, dst):
    """Copy bytes from one file descriptor to another until the source closes."""
    try:
        while True:
            data = os.read(src, 65536)
            if not data:
                break
            os.write(dst, data)
    except OSError:
        pass


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", default=os.path.join(os.environ.get("TMPDIR", "/tmp"), "ucs-headless.yaml"))
    ap.add_argument("--host", default=os.path.join(
        os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "ucs-host"))
    ap.add_argument("--size", default="80x25")
    ap.add_argument("--clusters", choices=["on", "off"], help="set mode 2027 before ucs-detect starts "
                    "(default: the engine's own)")
    ap.add_argument("rest", nargs="*", help="more arguments for ucs-detect")
    args = ap.parse_args()
    cols, rows = (int(v) for v in args.size.split("x"))
    env = suites_env()
    if os.path.realpath(sys.executable) != os.path.realpath(env["SUITES_PYTHON"]):
        os.execv(env["SUITES_PYTHON"], [env["SUITES_PYTHON"], *sys.argv])  # the venv has PyYAML
    host = subprocess.Popen([args.host, str(cols), str(rows), *([args.clusters] if args.clusters else [])], stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    pid, master = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, cols * 7, rows * 17))
        child_env = dict(os.environ, TERM="xterm-256color", COLORTERM="truecolor", PYTHONDONTWRITEBYTECODE="1",
                         PYTHONPATH=env["UCS_DETECT_DIR"])
        os.execve(env["SUITES_PYTHON"], [
            env["SUITES_PYTHON"], "-c", "import ucs_detect; ucs_detect.main()", "--save-yaml", args.out,
            "--set-software-name", "wormhole-headless", "--set-software-version", "headless",
            "--no-final-summary", *args.rest], child_env)
    # The pty's master side is the terminal's screen: what the program writes goes into the engine,
    # what the engine answers is typed back to the program.
    t_out = threading.Thread(target=pump, args=(master, host.stdin.fileno()), daemon=True)
    t_in = threading.Thread(target=pump, args=(host.stdout.fileno(), master), daemon=True)
    t_out.start(), t_in.start()
    _, status = os.waitpid(pid, 0)
    host.stdin.close()
    host.wait()
    if status != 0:
        sys.exit(f"ucs-detect exited with status {status}")
    report(args.out, env)


def report(path, env):
    import yaml

    with open(path, encoding="utf-8") as f:
        doc = yaml.safe_load(f)
    results = doc.get("test_results") or {}
    modes = (doc.get("terminal_results") or {}).get("modes") or {}
    for label, key in CATEGORIES:
        errors = total = 0
        for v in (results.get(key) or {}).values():
            if isinstance(v, dict):
                errors += int(v.get("n_errors", 0))
                total += int(v.get("n_total", 0))
        share = f"{100.0 * (total - errors) / total:.1f}%" if total else "-"
        print(f"{label:15} {share:>7}  ({total - errors}/{total})")
    m = modes.get(2027) or {}
    print(f"{'mode 2027':15} {'on' if m.get('enabled') else 'off'}")
    print(f"details: {path}")


if __name__ == "__main__":
    main()
