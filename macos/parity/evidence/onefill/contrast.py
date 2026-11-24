#!/usr/bin/env python3
"""Which colour the one fill (`one_background`) should be: `surface.window` or `sidebar.background`.

Today the main window paints three colours: `surface.window` (the terminal's column), `sidebar.background` (the sidebar and the
status bar) and `surface.panel` (the right panel). With the one fill all of them are one token, and every text and mark that sat
on the other two sits on it. Three measurements, over the 55 bundled themes (`macos/swift/Sources/DesignSystem/Resources/themes.json`):

1. Core's `CONTRAST_PAIRS` (parsed from `contrast.rs`, so the list is exactly what the theme rules hold) measured as core measures
   them (`ThemeTokens::opaque`): a pair changes when its background, or an underlay it names itself, is one of the three
   surfaces. Normal and high-contrast sets (Reduce Transparency uses the first, Increase Contrast the second).
2. The same at the default look (opacity 0.8, blur): the text colours `ThemeTokens::translucent` lifts, on the worst desktop it
   models (white behind a dark theme, black behind a light one), on what they sit on now and on the fill.
3. The terminal's card (`terminal.background`) against the fill around it, opaque and at 0.8.

Usage: python3 -I contrast.py [-v]    (-v lists every measurement that fell)
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
THEMES = json.loads((ROOT / "macos/swift/Sources/DesignSystem/Resources/themes.json").read_text())["themes"]
APPEARANCE = {t["id"]: t["appearance"] for t in THEMES}
CANDIDATES = ("surface.window", "sidebar.background")
SURFACES = {"surface.window", "sidebar.background", "surface.panel"}
OPACITY = 0.8
MATERIAL = {"dark": [28 / 255, 28 / 255, 30 / 255, 0.45], "light": [244 / 255, 244 / 255, 246 / 255, 0.45]}
VERBOSE = "-v" in sys.argv


def over(c, b):
    return [c[i] * c[3] + b[i] * (1 - c[3]) for i in range(3)] + [1.0]


def luminance(c):
    lin = lambda v: v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])


def ratio(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def core_pairs():
    text = (ROOT / "crates/wormhole-core/src/theme/tokens/contrast.rs").read_text()
    body = text[text.index("pub const CONTRAST_PAIRS"):text.index("impl ThemeTokens")]
    pairs = []
    for m in re.finditer(r'\b(pair|on)\(\s*((?:"[^"]*"\s*,\s*)+)([0-9.]+)\s*\)', body):
        names = re.findall(r'"([^"]*)"', m.group(2))
        fg, bg, under = (names + [None])[:3] if m.group(1) == "pair" else names
        pairs.append((fg, bg, under, float(m.group(3))))
    return pairs


def measure(ui, fg, bg, under):
    under = under or "surface.window"
    if not all(k in ui for k in (fg, bg, under)):
        return None
    window = over(ui["surface.window"], [0, 0, 0, 1])
    below = over(ui[bg], over(ui[under], window))
    return ratio(over(ui[fg], below), below)


def opaque(fill):
    rows = []
    for theme in THEMES:
        for name in ("normal", "highContrast"):
            ui = theme[name]["ui"]
            for fg, bg, under, minimum in core_pairs():
                if bg not in SURFACES and under not in SURFACES:
                    continue
                new_bg, new_under = (fill if bg in SURFACES else bg), (fill if under in SURFACES else under)
                if (new_bg, new_under) == (bg, under):
                    continue
                today, now = measure(ui, fg, bg, under), measure(ui, fg, new_bg, new_under)
                if today is not None and now is not None:
                    rows.append((theme["id"], name, fg, bg, under, minimum, today, now))
    return rows


def translucent(fill):
    rows = []
    for theme in THEMES:
        lifted = theme["normal"]["translucent"]
        if lifted is None:
            continue
        ui, dark = theme["normal"]["ui"], theme["appearance"] == "dark"
        desktop = [1, 1, 1, 1] if dark else [0, 0, 0, 1]
        behind = over(MATERIAL["dark" if dark else "light"], desktop)
        seen = lambda s: over(s[:3] + [s[3] * OPACITY], behind)
        win, side, pan = seen(ui["surface.window"]), seen(ui["sidebar.background"]), seen(ui["surface.panel"])
        band, hover, chip = ui["band"], ui["state.hover"], ui["git.branch.background"]
        opaque_window = over(ui["surface.window"], [0, 0, 0, 1])
        row, selected = ui["sidebar.row.selected"], ui["state.selected"]
        banded_now = [win, side, pan, over(band, side), over(band, pan)]
        hovered_now = banded_now + [over(hover, side), over(hover, pan)]
        chips_now = [win, side, pan, over(chip, over(band, side)), over(chip, over(band, pan))]
        selected_now = [over(row, over(ui["sidebar.background"], opaque_window)), over(selected, pan)]
        f = seen(ui[fill])
        banded_new = [f, over(band, f)]
        hovered_new = banded_new + [over(hover, f)]
        chips_new = [f, over(chip, over(band, f))]
        selected_new = [over(row, over(ui[fill], opaque_window)), over(selected, f)]
        for token, colour, minimum, now, new in [
            ("text.primary", lifted["text"], 4.5, hovered_now, hovered_new),
            ("text.muted", lifted["muted"], 4.5, banded_now, banded_new),
            ("text.faint", lifted["faint"], 3.0, hovered_now, hovered_new),
            ("git.branch", lifted["branch"], 4.5, chips_now, chips_new),
            ("sidebar.row.selected.text", lifted["selectedText"], 4.5, selected_now, selected_new),
            ("sidebar.row.selected.muted", lifted["selectedMuted"], 4.5, selected_now, selected_new),
        ]:
            c = colour[:3] + [1.0]
            rows.append((theme["id"], token, minimum, min(ratio(c, b) for b in now), min(ratio(c, b) for b in new)))
    return rows


def cards(fill):
    result = {}
    for theme in THEMES:
        card = theme["terminal"]["background"]
        for name in ("normal", "highContrast"):
            colour = theme[name]["ui"][fill]
            plain = ratio(over(card, [0, 0, 0, 1]), over(colour, [0, 0, 0, 1]))
            worst = None
            for desktop in ([1, 1, 1, 1], [0, 0, 0, 1]):
                behind = over(MATERIAL[APPEARANCE[theme["id"]]], desktop)
                around = over(colour[:3] + [colour[3] * OPACITY], behind)
                r = ratio(over(card[:3] + [card[3] * OPACITY], around), around)
                worst = r if worst is None else min(worst, r)
            result[(theme["id"], name)] = (plain, worst)
    return result


def main():
    for fill in CANDIDATES:
        rows = opaque(fill)
        bad = [r for r in rows if r[7] < r[6] - 0.005 and r[7] < r[5]]
        print(f"=== 1. opaque, fill = {fill}: {len(rows)} changed measurements; lower than today and below the pair's minimum: "
              f"{len(bad)} in {len({r[0] for r in bad})} themes ({len({r[0] for r in bad if APPEARANCE[r[0]] == 'light'})} light); "
              f"of those already below the minimum today: {sum(1 for r in bad if r[6] < r[5])}")
        by = defaultdict(list)
        for r in bad:
            by[(r[2], r[3], r[4], r[5])].append(r)
        for (fg, bg, under, minimum), rs in sorted(by.items(), key=lambda kv: -len(kv[1])):
            w = min(rs, key=lambda r: r[7])
            print(f"  {fg} on {bg}{' over ' + under if under else ''} (min {minimum}): {len(rs)}, worst {w[0]}/{w[1]} {w[6]:.2f} -> {w[7]:.3f}")
            if VERBOSE:
                for r in sorted(rs, key=lambda r: r[7]):
                    print(f"      {r[0]:30s} {r[1]:12s} {r[6]:5.2f} -> {r[7]:5.3f}")
        under_now = [r for r in rows if r[7] < r[5]]
        lower = [r for r in rows if r[7] < r[6] - 0.005 and r[7] >= r[5]]
        print(f"  below their minimum with this fill, whatever they were: {len(under_now)}; lower than today but at or above the "
              f"minimum: {len(lower)} ({len({r[0] for r in lower if APPEARANCE[r[0]] == 'light'})} light themes)")
        if lower:
            t = min(lower, key=lambda r: r[7] - r[5])
            print(f"  tightest of those: {t[0]}/{t[1]} {t[2]} on {t[3]}: {t[6]:.3f} -> {t[7]:.3f} (min {t[5]})")
    print()
    for fill in CANDIDATES:
        rows = translucent(fill)
        bad = [r for r in rows if r[4] < r[3] - 0.005 and r[4] < r[2]]
        print(f"=== 2. default look (opacity {OPACITY}, blur), fill = {fill}: lower than today and below the minimum: {len(bad)}")
        by = defaultdict(list)
        for r in bad:
            by[r[1]].append(r)
        for token, rs in by.items():
            w = min(rs, key=lambda r: r[4])
            print(f"  {token}: {len(rs)} themes, worst {w[0]} {w[3]:.2f} -> {w[4]:.2f} (min {w[2]})")
            if VERBOSE:
                for r in sorted(rs, key=lambda r: r[4]):
                    print(f"      {r[0]:30s} {r[3]:5.2f} -> {r[4]:5.2f}")
    print()
    today = cards("surface.window")
    for fill in CANDIDATES:
        now = cards(fill)
        for index, label in ((0, "opaque"), (1, f"at {OPACITY}")):
            keys = [k for k in now if index == 0 or k[1] == "normal"]
            higher = sum(1 for k in keys if now[k][index] > today[k][index] + 0.0005)
            lower = [k for k in keys if now[k][index] < today[k][index] - 0.0005]
            print(f"=== 3. card vs the fill around it, {label}, fill = {fill}: higher than today {higher}, equal "
                  f"{len(keys) - higher - len(lower)}, lower {len(lower)}"
                  + "".join(f" ({k[0]}: {today[k][index]:.4f} -> {now[k][index]:.4f})" for k in lower))
    for theme in ("wormhole.dark", "wormhole.light"):
        print(f"    {theme}: card vs surface.window {today[(theme, 'normal')][0]:.4f} opaque / {today[(theme, 'normal')][1]:.4f} at 0.8; "
              f"vs sidebar.background {cards('sidebar.background')[(theme, 'normal')][0]:.4f} / {cards('sidebar.background')[(theme, 'normal')][1]:.4f}")


main()
