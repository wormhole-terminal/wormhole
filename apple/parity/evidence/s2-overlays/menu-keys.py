#!/usr/bin/env python3
"""menu-keys.py <x> [--run]: read the instance's menu bar (control `menus`), turn each item's key into a `key` spec and, with --run,
press it through the control folder (the injected key goes through the app's own dispatch) and print what ran. Items that quit,
hide the app or close its windows are listed, never pressed."""
import subprocess, sys, time, os, re
X = sys.argv[1]; RUN = "--run" in sys.argv
C = f"/tmp/so-{X}/ctl"
def ctl(cmd, wait=1.0):
    open(C + "/out.txt", "w").close()
    open(C + "/cmd.tmp", "w").write(cmd + "\n"); os.rename(C + "/cmd.tmp", C + "/cmd"); time.sleep(wait)
    return open(C + "/out.txt").read()
MODS = {"⌃": "ctrl", "⌥": "alt", "⇧": "shift", "⌘": "cmd"}
NAMES = {"↩": "return", "⌫": "backspace", "↓": "down", "↑": "up", "←": "left", "→": "right", "Space": "space"}
def spec(glyphs):
    mods = []; rest = glyphs
    while rest and rest[0] in MODS: mods.append(MODS[rest[0]]); rest = rest[1:]
    key = NAMES.get(rest, rest.lower())
    return "-".join(mods + [key])
SKIP = {"Quit Wormhole", "Hide Wormhole", "Hide Others", "Close Window", "Close Terminal", "Minimize", "Enter Full Screen", "Export Text As…", "Print…"}
menu = ""; rows = []
for line in ctl("menus").splitlines():
    if line.startswith("> "): continue
    if not line.startswith(" "): menu = line.strip(); continue
    m = re.match(r"^\s+(.*?)\s{2,}([⌃⌥⇧⌘]*[^\s⌃⌥⇧⌘]\S*)$", line)
    if m and "(disabled)" not in line and (re.match(r"[⌃⌥⇧⌘]", m.group(2)) or re.match(r"F\d+$|⇧F\d+$", m.group(2))): rows.append((menu, m.group(1), m.group(2)))
for menu, title, key in rows:
    if re.search(r"\(disabled\)", title): continue
    s = spec(key)
    if not RUN or title in SKIP: print(f"{menu} > {title}  {key}  key {s}" + ("  [not pressed]" if title in SKIP else "")); continue
    out = ctl("key " + s, 1.3).strip().splitlines()
    print(f"{menu} > {title}  {key}  -> {out[0] if out else '(no answer)'}")
    ctl("key escape", 0.4); ctl("key escape", 0.4)
