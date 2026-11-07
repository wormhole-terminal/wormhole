#!/usr/bin/env python3
"""Pixel comparison of one scenario in the GPUI app and the Swift app, rerunnable.

    visual-compare.py compare NAME GPUI.png SWIFT.png --out DIR [--origin X,Y] [--size W,H]
                              [--swift-origin X,Y] [--cell W,H] [--gpui-cell-w PX] [--search N] [--matte R,G,B]
    visual-compare.py table DIR          the results of every scenario compared into DIR, as markdown

GPUI.png is cropped (--origin, --size, device pixels; the terminal's top left in the GPUI window) and compared with
SWIFT.png at --swift-origin (default its top left: a grid-only shot). --search N tries every shift of up to N pixels and keeps the best: the shift is itself a
measurement (padding and baseline differences). Alpha is composited over --matte (default mid grey, so a translucent
window shows as a difference). GPUI's cells are fractional (16.8 px wide at 14 pt, 2x) and Swift's whole pixels (17), so
columns drift apart by 0.2 px each; --gpui-cell-w 16.8 stretches the GPUI crop to Swift's cell width before comparing,
and the drift itself is reported as the cell width difference, not as thousands of failing blocks. Writes DIR/NAME.png (GPUI | Swift | difference, failing blocks outlined),
DIR/NAME.json and merges the numbers into DIR/results.json.

What counts as the same. Swift draws text with CoreText and GPUI with its own glyph raster, so no pixel-exact match is
possible or wanted. The image is cut into blocks (the terminal's cells; 16 px for chrome). A block passes when its mean
colour is within TOLERANCE levels per channel in both apps: the same fill, and the same amount of ink where text is.
`pass` is the share of passing blocks; `fill` the share of blocks whose dominant colour agrees within 3 levels (solid
areas: backgrounds, swatches, selection); `mad` the mean absolute difference per channel over all pixels, and `mad_blur`
the same after a Gaussian blur of 1.5 px, which removes anti-aliasing and keeps shape, colour and placement.
Same = shift 0,0, pass >= 98 %, fill >= 99 %.
"""
import argparse
import json
import pathlib
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

TOLERANCE = 12
PASS_MIN, FILL_MIN = 98.0, 99.0


def load(path, matte):
    image = Image.open(path).convert("RGBA")
    backdrop = Image.new("RGBA", image.size, (*matte, 255))
    return Image.alpha_composite(backdrop, image).convert("RGB")


def blocks(array, cell):
    """(rows, cols, cell_h, cell_w, 3) view over the whole blocks of an image."""
    h, w = array.shape[0] // cell[1] * cell[1], array.shape[1] // cell[0] * cell[0]
    return array[:h, :w].reshape(h // cell[1], cell[1], w // cell[0], cell[0], 3).swapaxes(1, 2)


def dominant(grid):
    rows, cols = grid.shape[:2]
    packed = (grid[..., 0].astype(np.uint32) << 16 | grid[..., 1].astype(np.uint32) << 8 | grid[..., 2]).reshape(rows, cols, -1)
    out = np.zeros((rows, cols, 3), np.int16)
    for r in range(rows):
        for c in range(cols):
            values, counts = np.unique(packed[r, c], return_counts=True)
            v = int(values[counts.argmax()])
            out[r, c] = (v >> 16, v >> 8 & 255, v & 255)
    return out


def best_shift(gpui, swift, origin, size, search):
    """The (dx, dy) that lines the GPUI crop up with Swift's top left, by luminance difference."""
    gray = lambda a: a.astype(np.float32).mean(axis=2)
    reference = gray(swift)
    best = (1e9, 0, 0)
    for dy in range(-search, search + 1):
        for dx in range(-search, search + 1):
            x, y = origin[0] + dx, origin[1] + dy
            if x < 0 or y < 0 or y + size[1] > gpui.shape[0] or x + size[0] > gpui.shape[1]:
                continue
            score = np.abs(gray(gpui[y : y + size[1], x : x + size[0]]) - reference).mean()
            best = min(best, (score, dx, dy))
    return best[1], best[2]


def pair(a, b):
    h, w = min(a.shape[0], b.shape[0]), min(a.shape[1], b.shape[1])
    return a[:h, :w], b[:h, :w]


def compare(args):
    matte = tuple(int(v) for v in args.matte.split(","))
    cell = tuple(int(v) for v in args.cell.split(","))
    origin = tuple(int(v) for v in args.origin.split(","))
    gpui_full, swift_full = np.asarray(load(args.gpui, matte)), np.asarray(load(args.swift, matte))
    size = tuple(int(v) for v in args.size.split(",")) if args.size else (swift_full.shape[1], swift_full.shape[0])
    so = tuple(int(v) for v in args.swift_origin.split(","))
    swift_full = swift_full[so[1] :, so[0] :]
    dx, dy = best_shift(gpui_full, swift_full[: size[1], : size[0]], origin, size, args.search) if args.search else (0, 0)
    x, y = origin[0] + dx, origin[1] + dy
    crop_w = round(size[0] * args.gpui_cell_w / cell[0]) if args.gpui_cell_w else size[0]
    gpui = gpui_full[y : y + size[1], x : x + crop_w]
    if crop_w != size[0]:
        gpui = np.asarray(Image.fromarray(gpui).resize(size, Image.BILINEAR))
    gpui, swift = pair(gpui, swift_full[: size[1], : size[0]])

    diff = np.abs(gpui.astype(np.int16) - swift.astype(np.int16))
    blur = lambda a: np.asarray(Image.fromarray(a).filter(ImageFilter.GaussianBlur(1.5))).astype(np.int16)
    mad_blur = float(np.abs(blur(gpui) - blur(swift)).mean())
    ga, sa = blocks(gpui, cell), blocks(swift, cell)
    gm, sm = ga.reshape(*ga.shape[:2], -1, 3).mean(axis=2), sa.reshape(*sa.shape[:2], -1, 3).mean(axis=2)
    passing = np.abs(gm - sm).max(axis=2) <= TOLERANCE
    fill = np.abs(dominant(ga) - dominant(sa)).max(axis=2) <= 3
    result = {
        "name": args.name, "shift": [dx, dy], "size": [gpui.shape[1], gpui.shape[0]], "cell": list(cell),
        "pass": round(float(passing.mean() * 100), 2), "fill": round(float(fill.mean() * 100), 2),
        "mad": round(float(diff.mean()), 3), "mad_blur": round(mad_blur, 3), "blocks": int(passing.size),
        "failing": [[int(r), int(c)] for r, c in zip(*np.nonzero(~passing))][:200],
    }
    result["same"] = (dx, dy) == (0, 0) and result["pass"] >= PASS_MIN and result["fill"] >= FILL_MIN

    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    heat = Image.fromarray(np.clip(diff * 4, 0, 255).astype(np.uint8))
    draw = ImageDraw.Draw(heat)
    for r, c in zip(*np.nonzero(~passing)):
        draw.rectangle([c * cell[0], r * cell[1], (c + 1) * cell[0] - 1, (r + 1) * cell[1] - 1], outline=(255, 64, 64))
    sheet = Image.new("RGB", (gpui.shape[1] * 3 + 16, gpui.shape[0]), (60, 60, 60))
    for i, image in enumerate((Image.fromarray(gpui), Image.fromarray(swift), heat)):
        sheet.paste(image, (i * (gpui.shape[1] + 8), 0))
    sheet.save(out / f"{args.name}.png")
    (out / f"{args.name}.json").write_text(json.dumps(result))
    merged = json.loads((out / "results.json").read_text()) if (out / "results.json").exists() else {}
    merged[args.name] = {k: v for k, v in result.items() if k != "failing"}
    (out / "results.json").write_text(json.dumps(merged, indent=1, sort_keys=True))
    print(json.dumps({k: v for k, v in result.items() if k != "failing"}))


def table(args):
    results = json.loads((pathlib.Path(args.dir) / "results.json").read_text())
    print("| scenario | shift px | pass % | fill % | mad | mad blur | same |\n|---|---|---|---|---|---|---|")
    for name, r in sorted(results.items()):
        print(f"| {name} | {r['shift'][0]},{r['shift'][1]} | {r['pass']} | {r['fill']} | {r['mad']} | {r['mad_blur']} | {'yes' if r['same'] else 'no'} |")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(required=True)
    c = sub.add_parser("compare")
    c.add_argument("name"); c.add_argument("gpui"); c.add_argument("swift")
    c.add_argument("--out", required=True); c.add_argument("--origin", default="0,0"); c.add_argument("--size")
    c.add_argument("--cell", default="16,16"); c.add_argument("--search", type=int, default=0)
    c.add_argument("--matte", default="128,128,128"); c.add_argument("--swift-origin", default="0,0"); c.add_argument("--gpui-cell-w", type=float)
    c.set_defaults(run=compare)
    t = sub.add_parser("table")
    t.add_argument("dir")
    t.set_defaults(run=table)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    sys.exit(main())
