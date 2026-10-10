#!/usr/bin/env python3
"""Compare Wormhole screenshots: changed pixels, where, and pictures of the difference.

    diff.py pair <before.png> <after.png> --out <dir> --name <name> [--scale 2] [--labels before,after]
    diff.py dirs <before dir> <after dir> --out <dir> [--scale 2]

A pixel counts as changed when any channel (alpha included) differs by more than --tolerance
levels (default 8 of 255), which ignores antialiasing noise but not a colour step. For each pair
it prints one line:

    <name>: <n> px changed (<pct>%), bbox 1x x0,y0-x1,y1, scale 2

and writes <name>-overlay.png (the after picture greyed, changed pixels red) and <name>-side.png
(before | after | overlay), both at 1x: the screenshots are at the display's scale (2 on Retina),
layout numbers are in logical pixels. Translucent
window areas are drawn over --backdrop (default black).

`dirs` compares every PNG present in both folders under the same name; it exits with 1 when any
of them differ, so it doubles as a determinism check. Needs Pillow and numpy
(`python3 -m pip install --user pillow numpy`); without them it says so and exits with 3.
"""

import argparse
import os
import sys

try:
    import numpy as np
    from PIL import Image, ImageDraw, ImageFont
except ImportError as missing:  # pragma: no cover - depends on the machine
    print(f"diff.py: {missing.name} is missing; install Pillow and numpy "
          "(python3 -m pip install --user pillow numpy)", file=sys.stderr)
    sys.exit(3)

LABEL_HEIGHT = 22
GAP = 8


def load(path):
    return Image.open(path).convert("RGBA")


def flatten(image, backdrop):
    """The picture as it would look over an opaque backdrop."""
    base = Image.new("RGBA", image.size, backdrop)
    return Image.alpha_composite(base, image).convert("RGB")


def changed_mask(before, after, tolerance):
    """Changed pixels of two pictures; pictures of different sizes are compared on their overlap,
    and everything outside it counts as changed."""
    width, height = max(before.width, after.width), max(before.height, after.height)
    a = np.zeros((height, width, 4), dtype=np.int16)
    b = np.zeros((height, width, 4), dtype=np.int16)
    a[: before.height, : before.width] = np.asarray(before, dtype=np.int16)
    b[: after.height, : after.width] = np.asarray(after, dtype=np.int16)
    mask = (np.abs(a - b) > tolerance).any(axis=2)
    if before.size != after.size:
        mask[min(before.height, after.height):, :] = True
        mask[:, min(before.width, after.width):] = True
    return mask


def bbox(mask):
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return None
    return int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1


def at_1x(image, scale):
    if scale == 1:
        return image
    size = (max(1, round(image.width / scale)), max(1, round(image.height / scale)))
    return image.resize(size, Image.LANCZOS)


def overlay(after, mask, backdrop):
    """`after` greyed out, with the changed pixels in red."""
    grey = flatten(after, backdrop).convert("L").convert("RGB")
    picture = Image.blend(Image.new("RGB", grey.size, (0, 0, 0)), grey, 0.45)
    height, width = mask.shape
    canvas = Image.new("RGB", (width, height), (0, 0, 0))
    canvas.paste(picture, (0, 0))
    red = Image.new("RGB", (width, height), (255, 40, 40))
    canvas.paste(red, (0, 0), Image.fromarray((mask * 255).astype(np.uint8)))
    return canvas


def fit_height(image, height):
    if image.height == height:
        return image
    width = max(1, round(image.width * height / image.height))
    return image.resize((width, height), Image.LANCZOS)


def label_font():
    try:
        return ImageFont.load_default(size=14)
    except TypeError:  # Pillow before 10.1 has one size
        return ImageFont.load_default()


def side_by_side(panels):
    """Pictures in a row, each under its label, at the height of the first."""
    height = panels[0][1].height
    panels = [(label, fit_height(image, height)) for label, image in panels]
    width = sum(image.width for _, image in panels) + GAP * (len(panels) - 1)
    canvas = Image.new("RGB", (width, height + LABEL_HEIGHT), (24, 24, 24))
    draw, font = ImageDraw.Draw(canvas), label_font()
    x = 0
    for label, image in panels:
        draw.text((x + 6, 3), label, fill=(230, 230, 230), font=font)
        canvas.paste(image.convert("RGB"), (x, LABEL_HEIGHT))
        x += image.width + GAP
    return canvas


def describe(name, mask, scale):
    count = int(mask.sum())
    pct = 100.0 * count / mask.size
    box = bbox(mask)
    where = "none" if box is None else "{},{}-{},{}".format(*(round(v / scale) for v in box))
    return f"{name}: {count} px changed ({pct:.3f}%), bbox 1x {where}, scale {scale:g}", count


def pair(before_path, after_path, out, name, scale, tolerance, backdrop, labels=("before", "after")):
    before, after = load(before_path), load(after_path)
    mask = changed_mask(before, after, tolerance)
    line, count = describe(name, mask, scale)
    os.makedirs(out, exist_ok=True)
    marked = at_1x(overlay(after, mask, backdrop), scale)
    marked.save(os.path.join(out, f"{name}-overlay.png"))
    panels = [(labels[0], at_1x(flatten(before, backdrop), scale)),
              (labels[1], at_1x(flatten(after, backdrop), scale)),
              (f"changed: {count} px", marked)]
    side_by_side(panels).save(os.path.join(out, f"{name}-side.png"))
    print(line)
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("mode", choices=["pair", "dirs"])
    parser.add_argument("first")
    parser.add_argument("second")
    parser.add_argument("--out", required=True)
    parser.add_argument("--name")
    parser.add_argument("--scale", type=float, default=2.0, help="device pixels per logical pixel")
    parser.add_argument("--tolerance", type=int, default=8, help="levels a channel may move unnoticed")
    parser.add_argument("--backdrop", default="#000000", help="colour behind translucent areas")
    parser.add_argument("--labels", default="before,after", help="what to call the two sides")
    args = parser.parse_args()

    if args.mode == "pair":
        labels = (args.labels.split(",", 1) + ["after"])[:2]
        pair(args.first, args.second, args.out, args.name or "shot", args.scale, args.tolerance,
             args.backdrop, labels)
        return 0
    names = sorted(set(n for n in os.listdir(args.first) if n.endswith(".png"))
                   & set(n for n in os.listdir(args.second) if n.endswith(".png")))
    if not names:
        print("diff.py: no PNG with the same name in both folders", file=sys.stderr)
        return 2
    differing = 0
    for png in names:
        count = pair(os.path.join(args.first, png), os.path.join(args.second, png), args.out,
                     png[:-4], args.scale, args.tolerance, args.backdrop)
        differing += count > 0
    print(f"{len(names)} compared, {differing} differ")
    return 1 if differing else 0


if __name__ == "__main__":
    sys.exit(main())
