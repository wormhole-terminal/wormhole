#!/usr/bin/env python3
"""The shortcut recorder rows (hotkey, jump key), driven by pointer and keys: record, refuse (an action's key, a macOS key, no
modifier), Esc, delete, the x, the reset arrow, and the hotkey held off while it records. Transcript to stdout."""
import sys
sys.argv = ["x", sys.argv[1] if len(sys.argv) > 1 else "b"]
exec(open(__file__.replace("recorder.py", "drive.py")).read().split("if __name__")[0])

def state(*keys):
    out = [run("hotkey")[-1]] + [f"{k} = {getval(k)}" for k in keys]
    print("   ", " | ".join(out), flush=True)

def start(x):
    """Click the field to record, with nothing recording before (a click on a recording field ends it, as the system's toggles do)."""
    if "CaptureView" in run("keys")[-1]:
        run("key esc")
    return f"click {cx(x)[0]:.0f} {cx(x)[1]:.0f}"

def step(title, *cmds):
    print(f"> {title}", flush=True)
    for line in run(*cmds, wait=6):
        print("   ", line[:200])
    time.sleep(0.5)

run("reset hotkey jump_waiting_key", "settings-section hotkey", "settings-size 920x1000", "pointer-window Settings"); time.sleep(1.5)
r = {x["id"]: x for x in rows()}
hk, jk = r["hotkey"], r["jump-waiting-key"]
cx = lambda x: (x["x"] + 60, x["y"] + x["h"] / 2)   # the field (the x clear button sits at the right end)
shot = lambda name: run(f"window-shot /tmp/s3shots/{name}.png Settings")

print("=== global hotkey row"); state("hotkey")
step("click the field: it records", start(hk), "keys"); state("hotkey"); shot("rec-recording")
step("press shift-cmd-space (the field takes it)", "key shift-cmd-space", "keys"); state("hotkey")
step("click again, press cmd-t (the app's New Terminal): refused with who has it", start(hk), "key cmd-t"); state("hotkey"); shot("rec-conflict-app")
step("press shift-cmd-4 (macOS: screenshots): refused", "key shift-cmd-4"); state("hotkey"); shot("rec-conflict-system")
step("press a bare j: wants a modifier", "key j"); state("hotkey")
step("Esc: recording ends, nothing changed", "key esc"); state("hotkey")
step("click, press alt-space (shell Meta): refused", start(hk), "key alt-space"); state("hotkey"); shot("rec-conflict-shell")
step("press ctrl-alt-cmd-h: accepted", "key ctrl-alt-cmd-h"); state("hotkey")
step("click, Backspace: clears", start(hk), "key backspace"); state("hotkey")
step("record again then the x button clears", start(hk), "key shift-cmd-space"); state("hotkey")
step("x (the clear button at the field's right end)", f"click {hk['x'] + hk['w'] - 14:.0f} {hk['y'] + hk['h'] / 2:.0f}"); state("hotkey")
step("hotkey-press with the hotkey set while recording: held off", "set hotkey=\"shift-cmd-space\"", start(hk), "hotkey"); shot("rec-paused")
step("Esc ends it: the hotkey is back", "key esc", "hotkey")
step("the reset arrow (reset hotkey) puts it back to none", "reset hotkey", "hotkey"); state("hotkey")

print("=== jump key row"); r = {x["id"]: x for x in rows()}; jk = r["jump-waiting-key"]; state("jump_waiting_key")
step("click, press alt-cmd-k", start(jk), "key alt-cmd-k"); state("jump_waiting_key")
step("click, press shift-j (shift alone types a capital: refused)", start(jk), "key shift-j"); state("jump_waiting_key"); shot("rec-jump-shift")
step("press cmd-t (the app's New Terminal): refused", "key cmd-t"); state("jump_waiting_key")
step("press alt-cmd-j (its own key: not a conflict)", "key alt-cmd-j"); state("jump_waiting_key")
step("the jump key works inside the app: hotkey-press jump", "reset jump_waiting_key")
