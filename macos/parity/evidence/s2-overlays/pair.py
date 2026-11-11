#!/usr/bin/env python3
"""pair.py <swift.png> <gpui.png> <out.png> [x0 y0 x1 y1]: the two captures side by side (cropped to the box, in pixels of the 2x shot),
Swift on the left, GPUI on the right, and the mean absolute difference of the crop (0 to 255) on stdout."""
import sys
from PIL import Image, ImageChops
a, b, out = sys.argv[1:4]
box = tuple(int(v) for v in sys.argv[4:8]) if len(sys.argv) >= 8 else None
A, B = Image.open(a).convert("RGB"), Image.open(b).convert("RGB")
if box: A, B = A.crop(box), B.crop(box)
W = max(A.width, B.width); H = max(A.height, B.height)
sheet = Image.new("RGB", (A.width + B.width + 16, H), (128, 128, 128)); sheet.paste(A, (0, 0)); sheet.paste(B, (A.width + 16, 0)); sheet.save(out)
if A.size == B.size:
    d = ImageChops.difference(A, B).convert("L"); px = list(d.getdata()); print(f"mean diff {sum(px)/len(px):.2f}, over 24: {100*sum(1 for v in px if v>24)/len(px):.2f} %")
