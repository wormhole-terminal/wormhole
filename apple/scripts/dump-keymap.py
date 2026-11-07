#!/usr/bin/env python3
"""Writes the Swift UI's keymap data from `wormhole-core`'s own source, so the menu bar, the shortcut
recorder and the palette never have a second table to drift from:

  keys/keymap/table.rs  -> every action: title, keys, scope, stage, flags (`entries`)
  keys/keymap/menus.rs  -> the menu bar's layout, as slots (`layout`)
  keys/keymap/mod.rs    -> the keys macOS keeps for itself (`systemKeys`)

The result is the PlatformModel resource `keymap.json`. `--check` changes nothing and fails when the
resource is out of date. It goes away when the FFI exposes `keymap::entries()` and `menus::menus()`
directly (FFI-CONTRACT: keymap). Run as `python3 -I apple/scripts/dump-keymap.py [--check]`.
"""
import json
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
keymap = root / "crates/wormhole-core/src/keys/keymap"
resource = root / "apple/swift/Sources/PlatformModel/Resources/keymap.json"


def strip_comments(text):
    return re.sub(r"(?m)^\s*//.*$|\s//[^\n\"]*$", "", text)


def camel(name):
    return name[0].lower() + name[1:]


def balanced(text, open_at):
    """The text between the parenthesis at `open_at` and its partner, and the index after the partner."""
    depth, i, in_string = 0, open_at, False
    while i < len(text):
        c = text[i]
        if in_string:
            if c == "\\":
                i += 1
            elif c == '"':
                in_string = False
        elif c == '"':
            in_string = True
        elif c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
            if depth == 0:
                return text[open_at + 1 : i], i + 1
        i += 1
    raise ValueError("unbalanced")


def split_top(text):
    parts, depth, in_string, start = [], 0, False, 0
    for i, c in enumerate(text):
        if in_string:
            in_string = not (c == '"' and text[i - 1] != "\\")
        elif c == '"':
            in_string = True
        elif c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == "," and depth == 0:
            parts.append(text[start:i].strip())
            start = i + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def string(token):
    return json.loads(token) if token.startswith('"') else token


def modifier_constants(table):
    out = {}
    for name, args in re.findall(r"const (\w+): KeyMods = mods\(([^)]*)\);", table):
        ctrl, alt, shift, cmd = [a.strip() == "true" for a in args.split(",")]
        out[name] = [m for m, on in (("ctrl", ctrl), ("alt", alt), ("shift", shift), ("cmd", cmd)) if on]
    return out


def parse_keys(token, mods):
    keys = []
    for pair in split_top(token.strip()[1:].strip()[1:-1]):  # `&[ ... ]`
        name, key = split_top(pair[1:-1])
        keys.append({"mods": mods[name], "key": string(key)})
    return keys


def parse_rows(table, mods):
    body = strip_comments(table[table.index("pub(super) fn entries()") :])
    digits = [str(n) for n in range(1, 10)]
    rows = []

    def parse_call(match_at):
        args, after = balanced(body, body.index("(", match_at))
        parts = split_top(args)
        row = {
            "id": camel(parts[0]),
            "title": string(parts[1]),
            "titleOff": None,
            "name": None,
            "keys": parse_keys(parts[2], mods),
            "scope": "app",
            "stage": "now",
            "palette": True,
            "debugOnly": False,
            "persistentOnly": False,
            "feature": None,
            "flag": None,
        }
        while True:
            chain = re.match(r"\s*\.(\w+)\(", body[after:])
            if not chain:
                break
            inner, end = balanced(body, after + chain.end() - 1)
            method, args = chain.group(1), split_top(inner)
            if method == "scope":
                row["scope"] = camel(args[0].split("::")[1])
            elif method == "system":
                row["stage"] = "system"
            elif method == "reserved":
                row["stage"] = "reserved"
            elif method == "not_in_palette":
                row["palette"] = False
            elif method == "toggle":
                row["titleOff"], row["name"] = string(args[0]), string(args[1])
            elif method == "named":
                row["name"] = string(args[0])
            elif method == "debug_only":
                row["debugOnly"] = True
            elif method == "persistent_only":
                row["persistentOnly"] = True
            elif method in ("flag", "feature"):
                row[method] = string(args[0])
            else:
                raise ValueError(f"unknown row method {method}")
            after = end
        return row, after

    at = 0
    while True:
        found = re.search(r"(?<![\w.])row\(\s*(?=[A-Z])", body[at:])
        if not found:
            break
        start = at + found.start()
        # The numbered loops build their rows from a title and a digit.
        row, at = parse_call(start)
        if row["id"].startswith(("terminal(", "group(")):
            continue
        rows.append(row)
    return rows, body, parse_call


def numbered_rows(body, mods):
    """`Terminal(n)` and `Group(n)`: pushed by loops over the title and digit lists."""
    rows = []
    for kind, scope_mods, stage_key in (("Terminal", "CMD", "terminal"), ("Group", "CTRL_ALT", "group")):
        titles = re.search(rf"const {kind.upper()}_TITLES: \[&str; 9\] =\s*\[(.*?)\];", TABLE, re.S)
        for n, title in enumerate(re.findall(r'"([^"]*)"', titles.group(1)), start=1):
            rows.append(
                {
                    "id": f"{stage_key}.{n}",
                    "title": title,
                    "titleOff": None,
                    "name": None,
                    "keys": [{"mods": mods[scope_mods], "key": str(n)}],
                    "scope": "app",
                    "stage": "now",
                    "palette": False,
                    "debugOnly": False,
                    "persistentOnly": False,
                    "feature": None,
                    "flag": None,
                }
            )
    return rows


def parse_slots(text, constants):
    slots = []
    for part in split_top(text):
        if part == "Sep":
            slots.append({"kind": "separator"})
        elif part.startswith("A("):
            slots.append({"kind": "action", "id": camel(part[2:-1])})
        elif part == "Slot::Services":
            slots.append({"kind": "services"})
        elif part.startswith("Slot::Submenu("):
            inner = split_top(part[len("Slot::Submenu(") : -1])
            slots.append(
                {"kind": "submenu", "title": string(inner[0]), "slots": parse_slots(inner[1].lstrip("&")[1:-1], constants)}
            )
        elif part.startswith("Slot::"):
            slots.append({"kind": "dynamic", "name": camel(part[len("Slot::") :])})
        else:
            raise ValueError(f"unknown slot {part}")
    return slots


def parse_layout(menus):
    text = strip_comments(menus)
    consts = {}
    for name, body in re.findall(r"const (\w+): &\[Slot\] =\s*&\[(.*?)\];", text, re.S):
        consts[name] = body
    menu_bar = re.search(r"let mut layout =\s*vec!\[(.*?)\];", text, re.S).group(1)
    layout = []
    for title, const in re.findall(r'\("(\w+)", (\w+)\)', menu_bar):
        layout.append({"title": title, "when": None, "slots": parse_slots(consts[const], consts)})
    extra = re.search(r'layout\.extend\(\[(.*?)\]\);', text, re.S).group(1)
    for title, const in re.findall(r'\("(\w+)", (\w+)\)', extra):
        layout.append({"title": title, "when": None, "slots": parse_slots(consts[const], consts)})
    debug = re.search(r'layout\.push\(\("(\w+)", (\w+)\)\)', text)
    layout.append({"title": debug.group(1), "when": "debug", "slots": parse_slots(consts[debug.group(2)], consts)})
    return layout


def parse_system_keys(mod):
    block = re.search(r"const SYSTEM_KEYS: &\[\(&str, &str\)\] = &\[(.*?)\];", mod, re.S).group(1)
    return [{"glyphs": g, "what": w} for g, w in re.findall(r'\("([^"]+)", "([^"]+)"\)', block)]


TABLE = (keymap / "table.rs").read_text()
mods = modifier_constants(TABLE)
rows, body, _ = parse_rows(TABLE, mods)
entries = rows + numbered_rows(body, mods)
# The table's order puts the numbered rows after Previous Terminal and Previous/Next Group; keep it.
order = [m for m in re.findall(r"(?<![\w.])row\(\s*([A-Z]\w*)", body)]
position = {}
for index, name in enumerate(order):
    position.setdefault(camel(name), index)
entries.sort(key=lambda e: (position[e["id"].split(".")[0]], e["id"]))

data = {
    "entries": entries,
    "layout": parse_layout((keymap / "menus.rs").read_text()),
    "systemKeys": parse_system_keys((keymap / "mod.rs").read_text()),
}
text = json.dumps(data, ensure_ascii=False, indent=1, sort_keys=True) + "\n"

if "--check" in sys.argv:
    if not resource.exists() or resource.read_text() != text:
        sys.exit("keymap.json is stale: run apple/scripts/dump-keymap.py")
    print("keymap.json is current")
else:
    resource.parent.mkdir(parents=True, exist_ok=True)
    resource.write_text(text)
    print(f"{len(entries)} actions, {len(data['layout'])} menus -> {resource}")
