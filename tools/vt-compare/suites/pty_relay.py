#!/usr/bin/env python3
"""Run a command on a pseudo-terminal and relay the terminal over this process's stdin and stdout.

For the headless esctest2 host (`esctest-host` in crates/wormhole-vt-bench): the host is the
terminal. What the command writes to its tty comes out on our stdout; what the host writes to our
stdin is typed into the tty. Keeping the PTY here means the Rust side needs no unsafe code.

usage: pty_relay.py ROWS COLS -- command [args...]
"""
import fcntl
import os
import pty
import select
import struct
import sys
import termios


def main():
    rows, cols = int(sys.argv[1]), int(sys.argv[2])
    cmd = sys.argv[sys.argv.index("--") + 1:]
    pid, master = pty.fork()
    if pid == 0:
        os.execvp(cmd[0], cmd)
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("hhhh", rows, cols, 0, 0))
    stdin, stdout = sys.stdin.fileno(), sys.stdout.fileno()
    open_stdin = True
    while True:
        fds = [master] + ([stdin] if open_stdin else [])
        ready, _, _ = select.select(fds, [], [])
        if master in ready:
            try:
                data = os.read(master, 65536)
            except OSError:
                data = b""
            if not data:
                break
            while data:
                data = data[os.write(stdout, data):]
        if stdin in ready:
            data = os.read(stdin, 65536)
            if not data:
                open_stdin = False
            while data:
                data = data[os.write(master, data):]
    _, status = os.waitpid(pid, 0)
    sys.exit(os.waitstatus_to_exitcode(status))


main()
