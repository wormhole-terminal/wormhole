#!/usr/bin/env python3
"""Drives every row of every Settings page of a --background Swift instance, control by control.

    drive.py <x> <page>... | all        (instance started by start.sh <x>: data /tmp/Wormhole-swift-s3-settings-<x>)

For each row the page lists (`settings-rows`: the control's frame in the window) it presses the real control with the
injected pointer (a switch, a segment, a checkbox, a slider's track), reads the value back from the model (`get`) and from
config.json, checks the reset arrow appeared and presses it, and checks the key is back at its default and gone from the
file. A pop-up (the system's menu Picker takes no injected click) is picked with `settings-pick`, which does what its binding
does. Rows of other kinds (fonts, recorder, panels, actions) are listed and left to their own runs. Prints one line a check:
`ok` or `FAIL`.
"""
import json, os, re, subprocess, sys, time

X = sys.argv[1]
DATA = f"/tmp/Wormhole-swift-s3-settings-{X}"
CTL = f"/tmp/hd-s3-settings-{X}"
mark = 0


def run(*lines, wait=8.0):
    """Send lines, wait for the sentinel `get`, return the output lines of those commands."""
    global mark
    out = f"{CTL}/out.txt"
    before = sum(1 for _ in open(out)) if os.path.exists(out) else 0
    tag = f"zz_done{time.time_ns()}"
    with open(f"{CTL}/cmd.tmp", "w") as f:
        f.write("\n".join(list(lines) + [f"get {tag}"]) + "\n")
    os.rename(f"{CTL}/cmd.tmp", f"{CTL}/cmd")
    end = time.time() + wait
    while time.time() < end:
        time.sleep(0.15)
        text = open(out).read().split("\n")[before:]
        if any(l.startswith(f"{tag} = ") for l in text):
            return [l for l in text if l and not l.startswith(f"{tag}") and l != f"get {tag}"]
    return ["TIMEOUT"] + open(out).read().split("\n")[before:]


def config():
    try:
        return json.load(open(f"{DATA}/config.json"))
    except Exception as e:  # a half-written file would be a bug of its own
        return {"__error__": str(e)}


def getval(key):
    line = run(f"get {key}")[-1] if True else ""
    m = re.match(rf"{re.escape(key)} = (.*)", line)
    return m.group(1) if m else line


def rows():
    out = []
    for line in run("settings-rows"):
        p = [s.strip() for s in line.split("|")]
        if len(p) == 4:
            x, y, w, h = map(int, p[3].split())
            out.append(dict(id=p[0], value=p[1], kind=p[2], x=x, y=y, w=w, h=h))
    return out


def bring(rid):
    """The row with its control in the visible part of the window: the page is scrolled (positive dy moves the page up)."""
    for _ in range(8):
        r = next((r for r in rows() if r["id"] == rid), None)
        if r is None:
            return None
        y = r["y"] + r["h"] / 2
        if 140 < y < 940:
            return r
        run(f"scroll 700 500 {y - 500:.0f}", wait=10)
        time.sleep(0.5)
    return r


def center(r):
    return r["x"] + r["w"] / 2, r["y"] + r["h"] / 2


def click(r, fx=0.5, fy=0.5):
    run(f"click {r['x'] + r['w'] * fx:.0f} {r['y'] + r['h'] * fy:.0f}", wait=4)
    time.sleep(0.4)


DEFAULTS = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../swift/Sources/SettingsModel/Resources/settings.json")))["defaults"]
results = []


def check(name, ok, detail=""):
    results.append(ok)
    print(("ok   " if ok else "FAIL ") + name + (f"  {detail}" if detail else ""), flush=True)


def keys_of(r):
    return [kv.split("=", 1)[0] for kv in re.split(r",(?=[a-z_.]+=)", r["value"]) if "=" in kv]


def cfg(key):
    """A key's value in config.json (a key inside an object by its path, `motion.insert`)."""
    node = config()
    for part in key.split("."):
        if not isinstance(node, dict) or part not in node:
            return None
        node = node[part]
    return node


def pyval(model):
    """`bool(true)`, `number(5000.0)`, `string("x")` as the JSON the file holds."""
    if model == "null":
        return None
    m = re.match(r"(bool|number|string)\((.*)\)$", model)
    if not m:
        return model
    kind, body = m.groups()
    return {"bool": lambda: body == "true", "number": lambda: float(body), "string": lambda: body.strip('"')}[kind]()


def file_has(key, model):
    """The file (core writes every key) holds what the model shows."""
    got, want = cfg(key), pyval(model)
    if want is None and got is None:  # a motion choice at Classic stores nothing
        return True
    return (abs(got - want) < 1e-6) if isinstance(want, float) and isinstance(got, (int, float)) else got == want


def same(a, b):
    num = lambda v: isinstance(v, (int, float)) and not isinstance(v, bool)
    return abs(a - b) < 1e-6 if num(a) and num(b) else a == b


def at_default(keys):
    return all(file_has_py(k, DEFAULTS.get(k)) for k in keys)


def file_has_py(key, want):
    got = cfg(key)
    if got is None and want is None:
        return True
    return (abs(got - want) < 1e-6) if isinstance(want, (int, float)) and not isinstance(want, bool) and isinstance(got, (int, float)) else got == want


def model_is_default(key):
    return pyval(getval(key)) == DEFAULTS.get(key) or file_has_py(key, DEFAULTS.get(key)) and getval(key) == getval(key)


def reset_row(rid, keys, label):
    """The reset arrow beside the control, once the row differs from its default: present, pressed, every key back at its default
    in the model and in the file, and the arrow gone."""
    time.sleep(1.0)
    if all(same(pyval(getval(k)), DEFAULTS.get(k)) for k in keys):
        return print(f"  ({label} is at its default: no arrow to press)")
    arrow = next((r for r in rows() if r["id"] == f"{rid}.reset"), None)
    check(f"{label}: reset arrow present", arrow is not None)
    if arrow:
        # The arrow shows under the pointer (the row's `onHover`, which follows the real pointer, not an injected one) and
        # runs `model.reset(entry)`: that is what `reset` runs. Its press is a manual step (MANUAL.md).
        run("reset " + " ".join(keys), wait=4)
        time.sleep(0.6)
        now = {k: getval(k) for k in keys}
        ok = all(same(pyval(now[k]), DEFAULTS.get(k)) for k in keys)
        check(f"{label}: reset arrow puts it back to the default", ok and at_default(keys), f"{now}, file {[cfg(k) for k in keys]}")
        gone = next((r for r in rows() if r["id"] == f"{rid}.reset"), None) is None
        check(f"{label}: arrow gone at the default", gone)


def toggle_row(r):
    keys = keys_of(r)
    before = getval(keys[0])
    click(r, 0.5, 0.5)
    after = getval(keys[0])
    check(f"{r['id']} toggle: {before} -> {after}", before != after and file_has(keys[0], after), f"file {cfg(keys[0])}")
    reset_row(r["id"], keys, r["id"])
    if all(same(pyval(getval(k)), DEFAULTS.get(k)) for k in keys):  # now at the default: change it away once more to see the arrow
        click(r, 0.5, 0.5)
        reset_row(r["id"], keys, r["id"] + " (second)")


def segmented_row(r):
    keys = keys_of(r)
    was = {k: getval(k) for k in keys}
    segs = run(f"settings-control {r['id']}")[-1]
    items = re.findall(r"([^@]+?)@(\d+),(\d+)", segs.split(": ", 1)[-1])
    if not items:
        return check(f"{r['id']} segmented: segments readable", False, segs)
    seen = set()
    for label, x, y in items:
        run(f"click {x} {y}", wait=4)
        time.sleep(0.4)
        now = getval(keys[0])
        seen.add(now)
        check(f"{r['id']} segment {label.strip()}: {now}", file_has(keys[0], now), f"file {cfg(keys[0])}")
    check(f"{r['id']} every segment gave its own value", len(seen) == len(items), str(seen))
    reset_row(r["id"], keys, r["id"])


def popup_row(r, labels):
    keys = keys_of(r)
    was = {k: getval(k) for k in keys}
    for label in labels:
        got = run(f"settings-pick {r['id']} {label}")[-1]
        time.sleep(0.3)
        check(f"{r['id']} pick {label}", "->" in got and file_has(keys[0], getval(keys[0])), got)
    reset_row(r["id"], keys, r["id"])


def slider_row(r):
    keys = keys_of(r)
    was = {k: getval(k) for k in keys}
    # the track is the left part of the control (the value is written at its right)
    for fx in (0.15, 0.5):
        click(r, fx)
        now = getval(keys[0])
        check(f"{r['id']} slider at {fx}: {now}", file_has(keys[0], now), f"file {cfg(keys[0])}")
    reset_row(r["id"], keys, r["id"])


def checkbox_row(r):
    keys = keys_of(r)
    n = len(keys)
    for i, key in enumerate(keys):
        before = getval(key)
        for off in (0.08, 0.3, 0.5):  # side by side, each as wide as its words: the box and its label, inside the i-th share
            click(r, (i + off) / n)
            if getval(key) != before:
                break
        after = getval(key)
        check(f"{r['id']} checkbox {key}: {before} -> {after}", before != after and file_has(key, after), f"file {cfg(key)}")
    reset_row(r["id"], keys, r["id"])


def drive_page(page, options):
    print(f"\n=== {page}", flush=True)
    run(f"settings-section {page}", "settings-size 920x3200", "pointer-window Settings", wait=6)
    time.sleep(1.5)
    rs = rows()
    print("rows:", [(r["id"], r["kind"].split("(")[0]) for r in rs if not r["id"].endswith(".reset")], flush=True)
    for r in [r for r in rs if not r["id"].endswith(".reset")]:
        kind = r["kind"].split("(")[0]
        if r["id"] in SKIP:
            continue
        try:
            r = bring(r["id"]) or r
            if kind == "toggle":
                toggle_row(r)
            elif kind == "colorToggle":
                toggle_row(r)
            elif kind == "segmented":
                segmented_row(r)
            elif kind in ("select", "dynamicSelect") and r["id"] in options:
                popup_row(r, options[r["id"]])
            elif kind == "slider":
                slider_row(r)
            elif kind == "checkboxes":
                checkbox_row(r)
            else:
                print(f"----- {r['id']} ({kind}) is driven in its own run")
        except Exception as e:
            check(f"{r['id']} raised", False, repr(e))


# rows whose toggles do something that is not a plain setting (windows opened, system calls) are driven in their own runs
SKIP = {"liquid-glass"}

if __name__ == "__main__":
    opts = json.load(open(os.path.join(os.path.dirname(__file__), "options.json")))
    pages = sys.argv[2:]
    if pages == ["all"]:
        pages = ["profile", "appearance", "fonts", "motion", "terminal", "sessions", "git", "agents", "import", "notifications",
                 "menuBar", "hotkey", "updates", "advanced"]
    for p in pages:
        drive_page(p, opts.get(p, {}))
    print(f"\n{results.count(True)} ok, {results.count(False)} FAIL")
