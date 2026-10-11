# ttystorm

Stress tests, benchmarks and full-screen animations for terminals. It runs in any terminal, so the
same bytes can be measured in Wormhole, Ghostty, iTerm2, kitty, WezTerm, Alacritty or Terminal.app,
and the animations are made to be recorded.

It is its own Cargo workspace with no dependencies, kept out of the app's build, lints and
dependency audit (`scripts/check.sh` skips it).

Every animation and every benchmark is a command of its own: `matrix`, `doomfire`, `logflood`,
`roundtrip`, … It is one program, `ttystorm`, that looks at the name it was started under, so each
command is a link to it (`matrix` and `ttystorm matrix` are the same thing).

```sh
brew install wormhole-terminal/tap/ttystorm   # ttystorm and a link for every command

matrix                                  # an animation; q to quit
logflood                                # a benchmark, with its result screen
roundtrip --plain                       # the same without the screen: plain text, for scripts
ttystorm list                           # every animation and benchmark
ttystorm showcase                       # all the animations, 10 s each, titled as each starts
ttystorm bench --save wormhole.json     # every benchmark, one result screen
ttystorm compare wormhole.json ghostty.json iterm2.json
```

From the source: `cargo build --release` in `tools/bench`, then link the commands somewhere on your
`PATH`:

```sh
for n in $(target/release/ttystorm names); do ln -sf "$PWD/target/release/ttystorm" ~/.local/bin/$n; done
```

Checks before a change here is done: `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and
`cargo test`, all in `tools/bench`.

## Animations

| Name | What | What it stresses |
|---|---|---|
| `warptunnel` | a fly-through tunnel, violet to white-hot core, with sparks | every cell's two truecolor values, every frame (~450 KB/frame at 200×69) |
| `plasma` | the demoscene plasma | the same, smooth gradients (banding shows) |
| `starfield` | warp-speed stars in braille, a hyperspace jump every 10 s | sparse braille, glyph and color changes everywhere |
| `doomfire` | the Doom fire; "WORMHOLE" burns in every 12 s | half-block truecolor with heavy churn at the bottom |
| `matrix` | digital rain in half-width katakana | many fg color changes, text glyph cache |
| `donut` | a spinning lit torus in ASCII, truecolor | per-cell fg + bg |
| `mandelbrot` | an endless zoom into seahorse valley | truecolor, the CPU side too (multi-threaded) |
| `life` | Conway's Game of Life in braille, colored by age | braille at full density |
| `unicode` | emoji (ZWJ, flags, skin tones), CJK, combining marks, RTL, box drawing | text shaping and widths: a terminal that counts a width differently shows it here |
| `dashboard` | a busy btop-style monitor with an agent log | a realistic TUI: boxes, sparklines, gradients |

Options: `--fps <n>` (default 60; `0` draws as fast as the terminal reads, the real stress test),
`--seconds <n>`, `--no-hud`, `--no-sync` (no synchronized output, mode 2026: shows tearing),
`--titles`. Keys: `q`/Esc quit, space pauses, `h` the overlay, `+ - 0` speed, `← →` previous/next.

The overlay in the top right shows the frames and bytes per second the terminal took in ("fps
sent"). It is **not** the frame rate on screen: a terminal reads its output on a thread of its own
and draws when it can, skipping frames it has already replaced, and no program can see from outside
how many it drew. 60 fps sent with a stuttering picture means the terminal reads fast but draws
slowly; that is a finding about the terminal, and the stutter is the thing to record. The number
only drops when even the reading falls behind (the program then blocks writing).

## Benchmarks

Each benchmark is a command (`logflood`, `roundtrip`, …); `ttystorm bench` runs them all. It writes
each workload's bytes (built in memory beforehand from a fixed seed, so
every terminal gets the same ones), then asks for the cursor position (DSR 6). A terminal answers
only after it has parsed everything before the question, so the clock stops when the work is really
done, not when the bytes reach the PTY. Each test runs 3 times (`--runs`) and reports the median.

| Command | Workload |
|---|---|
| `logflood` | plain log lines, like `cat` of a big file |
| `glyphstorm` | CJK, emoji, combining marks, box drawing |
| `colorflood` | every character with its own 24-bit fg and bg |
| `scrollstorm` | lines pushed through a DECSTBM region with reverse index, like vim or tmux |
| `cursorstorm` | random cursor moves with short colored writes, like a full-screen TUI |
| `longlines` | lines of 5–50 thousand characters that wrap |
| `framerate` | full-screen redraws in synchronized output, in frames per second |
| `roundtrip` | 1,000 cursor-report round trips on an idle terminal: p50, p90, p99 |

Options: `--plain` (no status line and no result screen, the results as plain text), `--size <MB>`
per test (default 16), `--runs <n>`, `--save <file.json>`, `--label <name>`, and for `ttystorm bench`
`--only logflood,roundtrip`. `compare` warns when the files differ in terminal size or machine:
`scrollstorm`, `cursorstorm`, `colorflood` and `framerate` depend on the size.

Up to 0.1, when ttystorm was called termbench, the benchmarks were `ascii`, `unicode`, `truecolor`,
`scroll-region`, `cursor`, `long-lines`, `frames` and `latency`. They send the same bytes under
their new names, files saved then (format `termbench`) still compare, and `--only` and `--kind` still
take the old names.

`ttystorm flood [--kind logflood|colorflood|glyphstorm|cursorstorm|…] [--seconds n]` writes one of these
workloads over and over, as fast as the terminal reads, until the time is up or `q` is pressed. It
measures nothing itself: it is the load for watching a terminal from outside, as `scripts/perf.sh`
does for Wormhole (draw time per frame, main-thread busy share and throughput against a baseline
kept in `baselines/`, see the header of that script).

What it doesn't measure: keypress-to-pixel latency (that needs a camera or a tool like
`typometer`), and frames actually drawn on screen.

## Recording for social media

- **Same conditions for every terminal**: the same window size (the overlay shows cols×rows), font
  size and machine, with nothing else busy. Quit other heavy apps; plug the laptop in.
- **Clean frame**: hide Wormhole's sidebar and panel, a dark theme, `--no-hud` unless the numbers are
  the point. macOS screen recording: ⇧⌘5, or `ffmpeg -f avfoundation` at 60 fps.
- **Loops**: `warptunnel --seconds 12 --no-hud` gives a clip with a clean start and end; the
  fire's word comes 2.5 s into every 12 s cycle.
- **Tearing vs not**: record `plasma --fps 0 --no-sync` next to `plasma --fps 0`.
- **Stress**: `--fps 0` with the overlay on shows how many frames and MB/s the terminal takes in;
  side by side with another terminal, it is the most direct comparison.
- **Numbers**: `ttystorm bench --save <name>.json` in each terminal, then
  `ttystorm compare a.json b.json …` for one screen with them all, best marked, ratios to the first
  file. The results screen paints its own background, so it looks the same in any theme.
