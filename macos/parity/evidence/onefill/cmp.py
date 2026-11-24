"""base-* (the build before the change) against new-*-off-* (the change, the setting off): every pixel, and every pixel outside the
terminal's card (a Metal view of its own, whose last easing to the palette can differ by a level or two between two runs)."""
import numpy as np
from PIL import Image
def load(p): return np.asarray(Image.open(p).convert("RGBA")).astype(int)
CARD = (slice(70, 1575), slice(710, 1910))  # y, x in pixels: a little outside the card (358..952pt, 38..784pt)
for ap in ("dark", "light"):
    for tag in ("op80", "op100"):
        a, b = load(f"shots/base-{ap}-{tag}.png"), load(f"shots/new-{ap}-off-{tag}.png")
        d = (a != b).any(axis=2)
        outside = d.copy(); outside[CARD] = False
        inside_max = int(np.abs(a - b)[CARD].max())
        print(f"{ap:5s} {tag:6s} {a.shape[1]}x{a.shape[0]}: differing pixels {int(d.sum()):8d} in all, {int(outside.sum())} outside the card; card's largest channel difference {inside_max}")
