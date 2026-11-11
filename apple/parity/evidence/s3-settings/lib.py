"""Shared helpers for the page drivers: `exec(open(lib.py).read())` after setting sys.argv = ['x', '<instance>']."""
import sys
exec(open(__file__.replace("lib.py", "drive.py")).read().split("if __name__")[0])

def say(title, *cmds, wait=6):
    print(f"> {title}", flush=True)
    for line in run(*cmds, wait=wait):
        print("   ", line[:220], flush=True)
    time.sleep(0.5)

def frame(rid):
    return next((r for r in rows() if r["id"] == rid), None)

def click_id(rid, fx=0.5, fy=0.5, mods=""):
    r = frame(rid)
    if r is None:
        print(f"   !! no frame {rid}", flush=True)
        return None
    run(f"click {r['x'] + r['w'] * fx:.0f} {r['y'] + r['h'] * fy:.0f} {mods}".strip(), wait=4)
    time.sleep(0.5)
    return r

def profiles():
    try:
        return json.load(open(f"{DATA}/profiles.json"))
    except Exception as e:
        return {"error": str(e)}

def shot(name, region=None):
    run(f"window-shot /tmp/s3shots/{name}.png Settings", wait=6)
    time.sleep(0.6)
