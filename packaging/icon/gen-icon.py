#!/usr/bin/env python3
"""Writes the Wormhole app icon: the Icon Composer bundle funnel.icon and the flat AppIcon.svg.

The icon is Terminal's `>_` over the textbook wormhole drawing: a wireframe funnel dipping into a
throat, in thin blue lines on a dark tile. Made the way macOS 26 icons are: one quiet tile, crisp
linework, glass only on the prompt.

funnel.icon (open it in Icon Composer.app) was the shipped icon until the lens icon replaced it: the
shipped one is now AppIcon.icon, which build-icns.sh compiles with actool into Assets.car (macOS 26+: Liquid
Glass, dark, tinted and clear looks) and AppIcon.icns (older macOS). To ship this one again, swap the two
folders' names. AppIcon.svg is this artwork flattened, still what the About window and the intro draw with
GPUI.

Tweak the glass, shadows and colors in Icon Composer if you like, but rerunning this script rewrites
icon.json, so carry such a change over into this file.

Groups (front to back): Prompt (glass `>_`), Funnel (flat lines), and the background fill.
Python 3.9 stdlib only (the macOS system python).

    python3 packaging/icon/gen-icon.py   # then, to ship it: swap funnel.icon and AppIcon.icon, run build-icns.sh
"""
import json
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
S = 1024

BG_TOP, BG_BOTTOM = "#141A36", "#05060F"
# The funnel is a glass layer that Icon Composer fills with LINES_P3, a mint green outside sRGB; LINES is
# its nearest sRGB colour, for the lines' own stroke and the flat SVG.
LINES_P3 = "display-p3:0.00000,1.00000,0.61051,1.00000"
LINES = "#00FF91"
PROMPT = "#E9EDF5"

# The funnel: centred, its rim just under the prompt, narrowing fast to a small throat near the bottom.
FUNNEL_CX, FUNNEL_TOP, FUNNEL_DEPTH = 512, 540, 380
RIM, THROAT, FLATTEN = 440, 40, 0.28
RINGS, SPOKES = 7, 12
# Icon Composer's placement of the funnel layer: scaled about the canvas centre, then moved down.
FUNNEL_SCALE, FUNNEL_SHIFT = 0.78, 40.0

# The prompt, sized and placed like Terminal's: top left, with room around it. The `>` is a round-capped
# polyline through CHEVRON, the `_` a line between CURSOR's ends, both STROKE wide. The intro's Voyage
# ship docks on this chevron (wormhole-core intro/voyage.rs reads these two lines in a test).
CHEVRON = ((214, 214), (289, 289), (214, 364))
CURSOR = ((352, 364), (457, 364))
STROKE = 46


def n(v):
    s = ("%.2f" % v).rstrip("0").rstrip(".")
    return "0" if s in ("", "-0") else s


def svg(body, defs=""):
    d = "<defs>%s</defs>" % defs if defs else ""
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">%s%s</svg>\n'
            % (S, S, S, S, d, body))


def line(x1, y1, x2, y2, w, color):
    return ('<line x1="%s" y1="%s" x2="%s" y2="%s" stroke="%s" stroke-width="%s" stroke-linecap="round"/>'
            % (n(x1), n(y1), n(x2), n(y2), color, n(w)))


def prompt(color):
    """`>_` as round-capped lines and a joint dot: in a glass layer actool closes an open polyline with a
    chord."""
    (ax, ay), (bx, by), (cx, cy) = CHEVRON
    (dx, dy), (ex, ey) = CURSOR
    return (line(ax, ay, bx, by, STROKE, color) + line(cx, cy, bx, by, STROKE, color)
            + '<circle cx="%s" cy="%s" r="%s" fill="%s"/>' % (n(bx), n(by), n(STROKE / 2), color)
            + line(dx, dy, ex, ey, STROKE, color))


def funnel():
    """Rings shrinking towards the throat and spokes down the wall; the back half of each spoke fainter."""
    def at(t):  # t = 0 at the rim .. 1 at the throat -> (radius, centre y)
        return RIM * (1 - t) ** 1.9 + THROAT, FUNNEL_TOP + FUNNEL_DEPTH * t ** 0.75

    parts = []
    for i in range(RINGS):
        t = i / (RINGS - 1.0)
        rad, y = at(t)
        parts.append('<ellipse cx="%s" cy="%s" rx="%s" ry="%s" fill="none" stroke="%s" stroke-opacity="%s" '
                     'stroke-width="%s"/>' % (n(FUNNEL_CX), n(y), n(rad), n(rad * FLATTEN), LINES,
                                              n(0.55 + 0.4 * t), n(11 - 3 * t)))
    for j in range(SPOKES):
        a = 2 * math.pi * j / SPOKES
        pts = []
        for i in range(31):
            rad, y = at(i / 30.0)
            pts.append((FUNNEL_CX + rad * math.cos(a), y + rad * FLATTEN * math.sin(a)))
        d = "M" + " L".join("%s,%s" % (n(x), n(y)) for x, y in pts)
        parts.append('<path d="%s" fill="none" stroke="%s" stroke-opacity="%s" stroke-width="6.5" '
                     'stroke-linejoin="round"/>' % (d, LINES, "0.3" if math.sin(a) < 0 else "0.7"))
    return "".join(parts)


# ------------------------------------------------------------------------------------------- icon.json
def color(hexstr, alpha=1.0):
    r, g, b = (int(hexstr[i:i + 2], 16) / 255.0 for i in (1, 3, 5))
    return "srgb:%.5f,%.5f,%.5f,%.5f" % (r, g, b, alpha)


def document():
    white = {"appearance": "tinted", "value": {"solid": "extended-gray:1.00000,1.00000"}}
    gradient = {"linear-gradient": [color(BG_TOP), color(BG_BOTTOM)]}
    return {
        "features": ["specular-location"],
        # The same tile in dark mode: without its own entry macOS swaps in a neutral grey one.
        "fill-specializations": [{"value": gradient}, {"appearance": "dark", "value": gradient}],
        "groups": [
            {"name": "Prompt",
             "layers": [{"name": "prompt", "image-name": "prompt.svg", "glass": True,
                         "fill-specializations": [white]}],
             "lighting": "combined", "specular": True, "shadow": {"kind": "neutral", "opacity": 0.25},
             "translucency": {"enabled": False, "value": 0}},
            {"name": "Funnel",
             "layers": [{"name": "funnel", "image-name": "funnel.svg", "glass": True,
                         "fill": {"solid": LINES_P3},
                         "position": {"scale": FUNNEL_SCALE, "translation-in-points": [0, FUNNEL_SHIFT]}}],
             "blur-material": 0.5, "lighting": "combined", "specular": "outside",
             "shadow": {"kind": "layer-color", "opacity": 0}, "translucency": {"enabled": True, "value": 0}},
        ],
        "supported-platforms": {"squares": "shared"},
    }


# -------------------------------------------------------------------------------------------- flat SVG
def flat_svg(funnel_body):
    """Both layers on the standard macOS grid (an 824 px body inside 1024), for GPUI's About window."""
    k = "%.6f" % (824.0 / 1024)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">\n'
        '  <!-- Wormhole app icon, flattened. Generated by gen-icon.py from the same layers as funnel.icon:\n'
        '       edit that script, not this file. -->\n'
        '  <defs>\n'
        '    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="%s"/>'
        '<stop offset="1" stop-color="%s"/></linearGradient>\n'
        '    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFFFFF" '
        'stop-opacity="0.22"/><stop offset="0.5" stop-color="#FFFFFF" stop-opacity="0.04"/><stop offset="1" '
        'stop-color="#FFFFFF" stop-opacity="0.08"/></linearGradient>\n'
        '    <clipPath id="body"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath>\n'
        '  </defs>\n'
        '  <rect x="100" y="100" width="824" height="824" rx="185" fill="url(#bg)"/>\n'
        '  <g clip-path="url(#body)"><g transform="translate(100 100) scale(%s)">'
        '<g transform="translate(512 %s) scale(%s) translate(-512 -512)">%s</g></g></g>\n'
        '  <rect x="102" y="102" width="820" height="820" rx="183" fill="none" stroke="url(#rim)" '
        'stroke-width="4"/>\n'
        '  <g transform="translate(100 100) scale(%s)">%s</g>\n'
        '</svg>\n'
        % (BG_TOP, BG_BOTTOM, k, n(512 + FUNNEL_SHIFT), n(FUNNEL_SCALE), funnel_body, k, prompt(PROMPT)))


def main():
    funnel_body = funnel()
    assets = {"prompt.svg": svg(prompt(PROMPT)), "funnel.svg": svg(funnel_body)}
    folder = os.path.join(HERE, "funnel.icon", "Assets")
    os.makedirs(folder, exist_ok=True)
    for name in os.listdir(folder):
        if name not in assets:
            os.remove(os.path.join(folder, name))
    for name, text in assets.items():
        with open(os.path.join(folder, name), "w") as f:
            f.write(text)
    with open(os.path.join(HERE, "funnel.icon", "icon.json"), "w") as f:
        # Icon Composer's own layout (sorted keys, " : ", no final newline): saving it there is a no-op.
        f.write(json.dumps(document(), indent=2, sort_keys=True, separators=(",", " : ")))
    with open(os.path.join(HERE, "AppIcon.svg"), "w") as f:
        f.write(flat_svg(funnel_body))
    print("wrote funnel.icon and AppIcon.svg")


if __name__ == "__main__":
    main()
