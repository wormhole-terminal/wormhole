#!/usr/bin/env python3
"""Scripted terminal content for the visual comparison, as the bytes a program would write.

    visual-scenes.py <dir>      writes <dir>/scenes/<name>.bin and <dir>/feed-<app>.sh

Both apps run the same feeder as their shell (`WORMHOLE_SHELL=<dir>/feed-gpui.sh`, `feed-swift.sh`): it shows a scene
when its name is written to `<dir>/<app>.fifo` (`echo text > <dir>/gpui.fifo`). Every scene starts with RIS so no state
leaks from the one before, and draws from the top left, where both apps anchor their grid.
"""
import base64
import pathlib
import struct
import sys
import zlib

ESC, ST = "\x1b", "\x1b\\"


def csi(s):
    return f"{ESC}[{s}"


def sgr(*codes):
    return csi(";".join(str(c) for c in codes) + "m")


RESET = sgr(0)


def at(row, col):
    return csi(f"{row};{col}H")


def line(row, text, col=1):
    return at(row, col) + text + RESET


def swatches(row, codes, label):
    return line(row, label.ljust(8) + "".join(sgr(*c) + f" {i:2} " + RESET for i, c in enumerate(codes)), 1)


def text_scene():
    out = [ESC + "c", csi("?25l"), line(1, sgr(1) + "Wormhole visual fixture " + sgr(0, 2) + "dim caption")]
    out.append(swatches(3, [(30 + i,) for i in range(8)], "fg 30"))
    out.append(swatches(4, [(90 + i,) for i in range(8)], "fg 90"))
    out.append(swatches(5, [(40 + i, 30 if i in (3, 6, 7) else 37) for i in range(8)], "bg 40"))
    out.append(swatches(6, [(100 + i, 30 if i in (3, 6, 7) else 37) for i in range(8)], "bg 100"))
    out.append(line(7, "256     " + "".join(sgr(48, 5, n) + " " for n in range(16, 232, 3))))
    out.append(line(8, "grey    " + "".join(sgr(48, 5, n) + "  " for n in range(232, 256))))
    out.append(line(9, "true    " + "".join(sgr(48, 2, int(i * 255 / 63), 80, 255 - int(i * 255 / 63)) + " " for i in range(64))))
    styles = [("bold", 1), ("dim", 2), ("italic", 3), ("reverse", 7), ("strike", 9), ("over", 53), ("hidden", 8)]
    out.append(line(11, "styles  " + " ".join(sgr(code) + name + RESET for name, code in styles)))
    out.append(line(12, "bold it " + sgr(1, 3) + "bold italic" + RESET + " " + sgr(1, 31) + "bold red" + RESET + " " + sgr(2, 32) + "dim green"))
    underlines = [("single", "4"), ("double", "4:2"), ("curly", "4:3"), ("dotted", "4:4"), ("dashed", "4:5")]
    out.append(line(13, "under   " + " ".join(csi(f"{code}m") + name + RESET for name, code in underlines)))
    out.append(line(14, "ul rgb  " + csi("4:3m") + csi("58;2;255;64;64m") + "curly red" + RESET + " "
                    + csi("4m") + csi("58;5;39m") + "single blue" + RESET + " " + sgr(4, 9) + "under+strike"))
    out.append(line(16, "wide    日本語のテキスト 한국어 中文 ＡＢＣ"))
    out.append(line(17, "emoji   🙂 🚀 👨‍👩‍👧 🇯🇵 ❤️ 👍🏽"))
    out.append(line(18, "marks   é ä ñ क्ष ab"))
    out.append(line(20, "box     ┌─┬─┐ ╔═╦═╗ ╭─╮ ┏━┓ │ │ ├─┼─┤ ╠═╬═╣ └─┴─┘ ╚═╩═╝ ╰─╯ ┗━┛"))
    out.append(line(21, "blocks  ░▒▓█ ▀▄▌▐ ▁▂▃▄▅▆▇█ ▖▗▘▙▚▛▜▝▞▟ ◢◣◤◥"))
    out.append(line(22, "braille ⠁⠃⠇⡇⣇⣧⣷⣿ ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"))
    out.append(line(23, "ligs    -> => == != >= <= === !== :: // /* */ |> <|> www"))
    out.append(line(24, "text    The quick brown fox jumps over the lazy dog 0123456789 {}[]()<>~`!@#$%^&*"))
    out.append(line(25, "link    " + f"{ESC}]8;;https://example.com{ST}an OSC 8 link{ESC}]8;;{ST} and a bare https://example.com/path"))
    out.append(line(26, "mixed   " + sgr(1, 38, 2, 255, 160, 0, 48, 2, 40, 40, 90) + " bold amber on slate " + RESET + " "
                    + sgr(3, 38, 5, 114) + "italic green" + RESET + " " + sgr(7, 31) + " reverse red " + RESET))
    return "".join(out)


def cursor_scene(style):
    out = [ESC + "c", line(1, "cursor style " + str(style)), line(2, "abcdefghij klmnop"), line(3, sgr(1, 31) + "wide 日本 here"),
           csi(f"{style} q"), at(2, 4)]
    return "".join(out)


def png_rgb(width, height):
    rows = b"".join(b"\x00" + bytes(c for x in range(width) for c in (x * 255 // (width - 1), y * 255 // (height - 1), 160))
                    for y in range(height))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def images_scene():
    data = base64.b64encode(png_rgb(96, 64)).decode()
    pieces = [data[i:i + 4000] for i in range(0, len(data), 4000)]
    kitty = "".join(f"{ESC}_G{'a=T,f=100,c=12,r=4,q=2,' if i == 0 else ''}m={0 if i == len(pieces) - 1 else 1};{p}{ST}"
                    for i, p in enumerate(pieces))
    iterm = f"{ESC}]1337;File=inline=1;width=12;height=4;preserveAspectRatio=0:{base64.b64encode(png_rgb(48, 32)).decode()}\x07"
    return "".join([ESC + "c", csi("?25l"), line(1, "kitty image, 12x4 cells below:"), at(2, 1), kitty,
                    line(7, "iTerm2 image, 12x4 cells below:"), at(8, 1), iterm, line(13, "text after the images")])


def blocks_scene():
    def block(command, code, output):
        return (f"{ESC}]133;A{ST}$ {ESC}]133;B{ST}{command}\r\n{ESC}]133;C{ST}{output}"
                f"{ESC}]133;D;{code}{ST}")

    return "".join([ESC + "c", block("ls src", 0, "main.rs\r\nlib.rs\r\n"), block("cargo test", 101, "running 3 tests\r\ntest a ... ok\r\ntest b ... FAILED\r\n"),
                    block("echo done", 0, "done\r\n"), f"{ESC}]133;A{ST}$ {ESC}]133;B{ST}"])


def scrollback_scene():
    rows = [f"{sgr(30 + i % 7 + 1)}line {i:03} {'#' * (i % 60)}{RESET}\r\n" for i in range(200)]
    return ESC + "c" + "".join(rows)


def pad_scene():
    """Grid edges: a character in every corner of a 40x12 block, to read cell metrics and padding."""
    out = [ESC + "c", csi("?25l")]
    for r in range(1, 13):
        out.append(line(r, ("0123456789" * 4)[:40]))
    out += [line(1, "┌" + "─" * 38 + "┐"), line(12, "└" + "─" * 38 + "┘")]
    out += [at(r, 1) + "│" + at(r, 40) + "│" for r in range(2, 12)]
    return "".join(out)


SCENES = {
    "text": text_scene,
    "cursor-block": lambda: cursor_scene(2),
    "cursor-underline": lambda: cursor_scene(4),
    "cursor-bar": lambda: cursor_scene(6),
    "images": images_scene,
    "blocks": blocks_scene,
    "scrollback": scrollback_scene,
    "pad": pad_scene,
}

FEED = """#!/bin/sh
# Shows the scene whose name is written to {fifo}; the terminal's program for the {app} app.
stty -echo
while :; do
  while read -r scene < {fifo}; do cat "{scenes}/$scene.bin"; done
done
"""


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    root = pathlib.Path(sys.argv[1]).resolve()
    scenes = root / "scenes"
    scenes.mkdir(parents=True, exist_ok=True)
    for name, make in SCENES.items():
        (scenes / f"{name}.bin").write_bytes(make().encode())
    for app in ("gpui", "swift"):
        fifo = root / f"{app}.fifo"
        if not fifo.exists():
            import os
            os.mkfifo(fifo)
        script = root / f"feed-{app}.sh"
        script.write_text(FEED.format(fifo=fifo, app=app, scenes=scenes))
        script.chmod(0o755)
    print(f"{len(SCENES)} scenes in {scenes}")


if __name__ == "__main__":
    main()
