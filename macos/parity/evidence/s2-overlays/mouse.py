#!/usr/bin/env python3
# A program that tracks the mouse the way a coding agent does: it draws its own selection, copies it with OSC 52 on release, and
# shows every byte it is sent (control bytes as ^X) so a paste over its selection can be read off the screen.
import os, sys, tty, base64
tty.setraw(0)
out = lambda s: (os.write(1, s.encode()), None)[1]
out("\x1b[?1000h\x1b[?1006h" + "selectable text of the program\r\n")
while True:
    b = os.read(0, 1)
    if not b: break
    c = b[0]
    out("^" + chr(c ^ 64) if c < 32 or c == 127 else chr(c))
    if b == b"m":  # SGR mouse release: the program copies what it selected
        out("\x1b]52;c;" + base64.b64encode(b"program-copy").decode() + "\x07")
