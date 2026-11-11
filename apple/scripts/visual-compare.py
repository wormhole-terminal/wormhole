#!/usr/bin/env python3
"""Pixel comparison of one scenario in the GPUI app and the Swift app, rerunnable.

    visual-compare.py compare NAME GPUI.png SWIFT.png --out DIR [--origin X,Y] [--size W,H] [--swift-origin X,Y]
                              [--cell W,H] [--search N] [--matte R,G,B] [--regions FILE --scene NAME [--first-row N]]
    visual-compare.py delta NAME GPUI_OFF GPUI_ON SWIFT_OFF SWIFT_ON --out DIR   where a setting changes each window, and the overlap
    visual-compare.py table DIR          the results of every scenario compared into DIR, as markdown

GPUI.png is cropped (--origin, --size, device pixels; the terminal's top left in the GPUI window) and compared with
SWIFT.png at --swift-origin (default its top left: a grid-only shot). --search N tries every shift of up to N pixels and keeps the best: the shift is itself a
measurement (padding and baseline differences). Alpha is composited over --matte (default mid grey, so a translucent
window shows as a difference). Both apps size the grid by the core's cell (`view::layout::cell`), whose width is fractional
(16.8 px at 14 pt, 2x): --cell takes it as it is, and block c spans the device pixels from round(c x w) to round((c + 1) x w),
where both apps put a cell's edge. Nothing is stretched and no pixel is left out. Writes DIR/NAME.png (GPUI | Swift |
difference, failing blocks outlined), DIR/NAME.json and merges the numbers into DIR/results.json.

What counts as the same. The image is cut into blocks (the terminal's cells; 16 px for chrome). A block passes when its mean
colour is within TOLERANCE levels per channel in both apps: the same fill, and the same amount of ink where text is.
`pass` is the share of passing blocks; `fill` the share of blocks whose dominant colour agrees within 3 levels (solid
areas: backgrounds, swatches, selection); `mad` the mean absolute difference per channel over all pixels, and `mad_blur`
the same after a Gaussian blur of 1.5 px, which removes anti-aliasing and keeps shape, colour and placement.
A scene-wide share cannot carry a feature (2 % of a grid is a hundred blocks, more than a cursor or an image covers), so a
scene names the cells each of its features occupies (--regions, written by visual-scenes.py: row0, col0, row1, col1 in the
scene's cells; --first-row when the crop starts lower) and every block of every region must pass; the failing blocks of
each region are listed in NAME.json and in the table.
Same = shift 0,0, pass >= 98 %, fill >= 99 %, every region 100 %.
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


def edges(extent, cell):
    """The pixel edges of whole cells of `cell` pixels (fractional) across `extent` pixels."""
    count = int(extent / cell + 1e-6)
    return [round(i * cell) for i in range(count + 1)]


def block_means(array, xs, ys):
    """(rows, cols, 3) mean colour of each block, from an integral image."""
    integral = np.zeros((array.shape[0] + 1, array.shape[1] + 1, 3), np.float64)
    integral[1:, 1:] = array.astype(np.float64).cumsum(0).cumsum(1)
    x, y = np.array(xs), np.array(ys)
    total = integral[y[1:, None], x[None, 1:]] - integral[y[:-1, None], x[None, 1:]] - integral[y[1:, None], x[None, :-1]] + integral[y[:-1, None], x[None, :-1]]
    area = ((y[1:] - y[:-1])[:, None] * (x[1:] - x[:-1])[None, :])[..., None]
    return total / area


def dominant(array, xs, ys):
    out = np.zeros((len(ys) - 1, len(xs) - 1, 3), np.int16)
    packed = array[..., 0].astype(np.uint32) << 16 | array[..., 1].astype(np.uint32) << 8 | array[..., 2]
    for r in range(len(ys) - 1):
        for c in range(len(xs) - 1):
            values, counts = np.unique(packed[ys[r]:ys[r + 1], xs[c]:xs[c + 1]], return_counts=True)
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
    cell = tuple(float(v) for v in args.cell.split(","))
    origin = tuple(int(v) for v in args.origin.split(","))
    gpui_full, swift_full = np.asarray(load(args.gpui, matte)), np.asarray(load(args.swift, matte))
    size = tuple(int(v) for v in args.size.split(",")) if args.size else (swift_full.shape[1], swift_full.shape[0])
    so = tuple(int(v) for v in args.swift_origin.split(","))
    swift_full = swift_full[so[1] :, so[0] :]
    dx, dy = best_shift(gpui_full, swift_full[: size[1], : size[0]], origin, size, args.search) if args.search else (0, 0)
    x, y = origin[0] + dx, origin[1] + dy
    gpui, swift = pair(gpui_full[y : y + size[1], x : x + size[0]], swift_full[: size[1], : size[0]])

    diff = np.abs(gpui.astype(np.int16) - swift.astype(np.int16))
    blur = lambda a: np.asarray(Image.fromarray(a).filter(ImageFilter.GaussianBlur(1.5))).astype(np.int16)
    mad_blur = float(np.abs(blur(gpui) - blur(swift)).mean())
    xs, ys = edges(gpui.shape[1], cell[0]), edges(gpui.shape[0], cell[1])
    passing = np.abs(block_means(gpui, xs, ys) - block_means(swift, xs, ys)).max(axis=2) <= TOLERANCE
    fill = np.abs(dominant(gpui, xs, ys) - dominant(swift, xs, ys)).max(axis=2) <= 3
    regions = {}
    if args.regions:
        named = json.loads(pathlib.Path(args.regions).read_text()).get(args.scene or args.name, {})
        for region, (r0, c0, r1, c1) in named.items():
            r0, r1 = max(0, r0 - args.first_row), min(passing.shape[0] - 1, r1 - args.first_row)
            c1 = min(passing.shape[1] - 1, c1)
            if r1 < r0 or c1 < c0:
                regions[region] = {"pass": None, "failing": "outside the crop"}
                continue
            part = passing[r0 : r1 + 1, c0 : c1 + 1]
            fails = [[int(r + r0 + args.first_row), int(c + c0)] for r, c in zip(*np.nonzero(~part))]
            regions[region] = {"pass": round(float(part.mean() * 100), 2), "failing": fails}
    result = {
        "name": args.name, "shift": [dx, dy], "size": [gpui.shape[1], gpui.shape[0]], "cell": list(cell),
        "pass": round(float(passing.mean() * 100), 2), "fill": round(float(fill.mean() * 100), 2),
        "mad": round(float(diff.mean()), 3), "mad_blur": round(mad_blur, 3), "blocks": int(passing.size),
        "regions": regions, "failing": [[int(r), int(c)] for r, c in zip(*np.nonzero(~passing))][:200],
    }
    result["same"] = ((dx, dy) == (0, 0) and result["pass"] >= PASS_MIN and result["fill"] >= FILL_MIN
                      and all(r["pass"] == 100.0 for r in regions.values()))

    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    heat = Image.fromarray(np.clip(diff * 4, 0, 255).astype(np.uint8))
    draw = ImageDraw.Draw(heat)
    for r, c in zip(*np.nonzero(~passing)):
        draw.rectangle([xs[c], ys[r], xs[c + 1] - 1, ys[r + 1] - 1], outline=(255, 64, 64))
    sheet = Image.new("RGB", (gpui.shape[1] * 3 + 16, gpui.shape[0]), (60, 60, 60))
    for i, image in enumerate((Image.fromarray(gpui), Image.fromarray(swift), heat)):
        sheet.paste(image, (i * (gpui.shape[1] + 8), 0))
    sheet.save(out / f"{args.name}.png")
    (out / f"{args.name}.json").write_text(json.dumps(result))
    merged = json.loads((out / "results.json").read_text()) if (out / "results.json").exists() else {}
    merged[args.name] = {k: v for k, v in result.items() if k != "failing"}
    (out / "results.json").write_text(json.dumps(merged, indent=1, sort_keys=True))
    print(json.dumps({k: v for k, v in result.items() if k != "failing"}))


def delta(args):
    """Where a setting changes each app's window (off and on shots of each), and whether the same places change in both."""
    matte = (128, 128, 128)
    def changed(off, on):
        a, b = np.asarray(load(off, matte)).astype(np.int16), np.asarray(load(on, matte)).astype(np.int16)
        h, w = min(a.shape[0], b.shape[0]), min(a.shape[1], b.shape[1])
        return np.abs(a[:h, :w] - b[:h, :w]).max(axis=2) > 3
    g, s = changed(args.gpui_off, args.gpui_on), changed(args.swift_off, args.swift_on)
    h, w = min(g.shape[0], s.shape[0]), min(g.shape[1], s.shape[1])
    g, s = g[:h, :w], s[:h, :w]
    def box(mask):
        ys, xs = np.nonzero(mask)
        return [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())] if len(xs) else None
    both, either = int((g & s).sum()), int((g | s).sum())
    result = {"name": args.name, "gpui_changed": int(g.sum()), "swift_changed": int(s.sum()), "gpui_box": box(g), "swift_box": box(s),
              "overlap": round(both / either, 3) if either else None}
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    picture = np.zeros((h, w, 3), np.uint8)
    picture[g] = (255, 64, 64)
    picture[s] += np.array((0, 200, 255), np.uint8)
    Image.fromarray(picture).save(out / f"{args.name}.png")
    (out / f"{args.name}.json").write_text(json.dumps(result))
    print(json.dumps(result))


def table(args):
    results = json.loads((pathlib.Path(args.dir) / "results.json").read_text())
    print("| scenario | shift px | pass % | fill % | mad | mad blur | regions (failing blocks: row,col) | same |\n|---|---|---|---|---|---|---|---|")
    for name, r in sorted(results.items()):
        regions = "; ".join(
            f"{k} {v['pass']}" + (f" {v['failing']}" if v["failing"] else "") for k, v in sorted(r.get("regions", {}).items()))
        print(f"| {name} | {r['shift'][0]},{r['shift'][1]} | {r['pass']} | {r['fill']} | {r['mad']} | {r['mad_blur']} | {regions or '-'} | {'yes' if r['same'] else 'no'} |")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(required=True)
    c = sub.add_parser("compare")
    c.add_argument("name"); c.add_argument("gpui"); c.add_argument("swift")
    c.add_argument("--out", required=True); c.add_argument("--origin", default="0,0"); c.add_argument("--size")
    c.add_argument("--cell", default="16,16"); c.add_argument("--search", type=int, default=0)
    c.add_argument("--matte", default="128,128,128"); c.add_argument("--swift-origin", default="0,0")
    c.add_argument("--regions"); c.add_argument("--scene"); c.add_argument("--first-row", type=int, default=0)
    c.set_defaults(run=compare)
    d = sub.add_parser("delta")
    d.add_argument("name"); d.add_argument("gpui_off"); d.add_argument("gpui_on"); d.add_argument("swift_off"); d.add_argument("swift_on")
    d.add_argument("--out", required=True)
    d.set_defaults(run=delta)
    t = sub.add_parser("table")
    t.add_argument("dir")
    t.set_defaults(run=table)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    sys.exit(main())
