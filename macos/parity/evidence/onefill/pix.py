"""pix.py shot.png [shot2.png ...]: the colour of each part of the window and whether a line lies between them.
Coordinates are in points of the 1320x820 window (the shot is 2x). Points chosen where nothing but the surface is drawn."""
import sys
import numpy as np
from PIL import Image

PTS = {  # name: (x, y) in points
    "sidebar": (130, 700),
    "column (outside the card)": (357, 400),   # the strip between the sidebar's edge and the card
    "panel": (1140, 700),
    "status bar": (700, 805),
    "card (terminal)": (650, 400),
}

def load(path):
    return np.asarray(Image.open(path).convert("RGBA")).astype(int)

def at(img, x, y):
    return tuple(int(v) for v in img[int(y * 2), int(x * 2)])

def report(path):
    img = load(path)
    h, w = img.shape[:2]
    print(path.split("/")[-1], f"{w}x{h}")
    for name, (x, y) in PTS.items():
        print(f"  {name:28s} {at(img, x, y)}")

def lines_between(path):
    """x positions (in points) along the row at y=592pt (through the sidebar, the card and the panel) where the colour changes."""
    img = load(path)
    row = img[int(592 * 2), 24:-24]
    changes = [i + 24 for i in range(1, len(row)) if tuple(row[i]) != tuple(row[i - 1])]
    return [c / 2 for c in changes]

def status_rule(path):
    """rows (points) at x=1000pt, below the panel's content, whose colour differs from the status bar's own fill."""
    img = load(path)
    fill = tuple(int(v) for v in img[int(812 * 2), 2000])
    out = []
    for y2 in range(int(760 * 2), int(820 * 2)):
        c = tuple(int(v) for v in img[y2, 2000])
        if c != fill:
            out.append((y2 / 2, c))
    return fill, out

if __name__ == "__main__":
    for p in sys.argv[1:]:
        report(p)
        print("  colour changes along y=592pt (x in points):", lines_between(p)[:12])
        fill, rule = status_rule(p)
        print("  status bar fill", fill, "| rows at x=1000pt that differ from it:", rule[:6])
