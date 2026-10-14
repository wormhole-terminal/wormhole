# ttystorm

Stress tests, benchmarks and full-screen animations for terminals. It runs in any terminal, so the
same bytes can be measured in Wormhole, Ghostty, iTerm2, kitty, WezTerm, Alacritty or Terminal.app,
and the animations are made to be recorded.

This is Wormhole's copy of [ttystorm](https://github.com/wormhole-terminal/ttystorm), kept in sync
with it. It is its own Cargo workspace with no dependencies, kept out of the app's build, lints and
dependency audit (`scripts/check.sh` skips it).

Every animation and every benchmark is a command of its own: `matrix`, `doomfire`, `logflood`,
`roundtrip`, … It is one program, `ttystorm`, that looks at the name it was started under, so each
command is a link to it (`matrix` and `ttystorm matrix` are the same thing).

```sh
brew install wormhole-terminal/tap/ttystorm   # ttystorm and a link for every command

matrix                                  # an animation; q to quit
logflood                                # a benchmark, with its result screen
roundtrip --plain                       # the same without the screen: plain text, for scripts
logflood --details                      # why: stalls, read vs parse time, the terminal's CPU and memory
ttyprobe                                # what the terminal supports and how wide it draws emoji and CJK
ttystorm list                           # every animation and benchmark
ttystorm showcase                       # all the animations, 10 s each, titled as each starts
ttystorm bench --save wormhole.json     # every benchmark, one result screen
ttystorm compare wormhole.json ghostty.json iterm2.json   # --markdown for a table to post
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
`--seconds <n>`, `--hud`, `--no-sync` (no synchronized output, mode 2026: shows tearing),
`--titles`. Keys: `q`/Esc quit, space pauses, `h` the overlay, `+ - 0` speed, `← →` previous/next.

An animation starts with nothing but the picture. `h` (or `--hud`) shows the overlay in the top
right: the frames and bytes per second the terminal took in ("fps sent"). It is **not** the frame
rate on screen: a terminal reads its output on a thread of its own and draws when it can, skipping
frames it has already replaced, and no program can see from outside how many it drew. 60 fps sent
with a stuttering picture means the terminal reads fast but draws slowly; that is a finding about
the terminal, and the stutter is the thing to record. The number only drops when even the reading
falls behind (the program then blocks writing).

## Benchmarks

Each benchmark is a command (`logflood`, `roundtrip`, …); `ttystorm bench` runs them all. It writes
each workload's bytes (built in memory beforehand from a fixed seed, so
every terminal gets the same ones), then asks for the cursor position (DSR 6). A terminal answers
only after it has parsed everything before the question, so the clock stops when the work is really
done, not when the bytes reach the PTY. Each test runs 3 times (`--runs`) and reports the median.

**Realistic**: what programs actually print. These are the numbers that predict how a terminal feels in daily use.

| Command | Workload | Why it is realistic |
|---|---|---|
| `buildlog` | a cargo-style build: `Compiling` lines with a bold green verb, warning and error blocks (`-->`, gutter, source line, colored `^^^^`), a progress bar redrawn in place with CR and EL, a `Finished` line | what `cargo build`, `make` or `npm install` print: plain text, a little color and in-place redraws |
| `agentstream` | an AI agent's answer: bold, inline code, bullets, a fenced code block with syntax colors, a diff, and a spinner line (`✻ Thinking… (12s · ↓ 1.2k tokens)`) redrawn under it, all as **2 to 16 byte writes** | an agent streams tokens, not blocks, so this measures what each write costs. The data is capped at 2 MB, because tiny writes are slow; the result is MB/s, and `writes_per_s` is kept in `extra` |
| `agentredraw` | a coding agent's classic UI on the main screen: a live region of 6 to 20 rows at the bottom (the reply's last rows, the spinner line, an input box, a hint line) **redrawn in full, in place, every frame** (CR, cursor up, each row erased and rewritten) as a few words arrive; finished replies are printed once above it, into the scrollback; every 200 frames the whole transcript is repainted (home, erase screen and scrollback, every row so far), as a classic renderer may after a resize, so these frames grow like a long session's. One synchronized update (mode 2026) and one write per frame, at most 1,200 frames | an agent's classic mode in a long session: a dozen styled rows resent every frame, and ever bigger repaints. The result is frames per second; `mbps` and `writes_per_s` are kept in `extra` |
| `agentscreen` | the same agent's fullscreen UI: the alternate screen with the input box pinned at the bottom, and only changed cells sent between frames (cursor positioning plus the spinner's glyph, seconds and tokens, the words added to the reply, now and then a typed character); when the reply reaches the bottom the message area scrolls by writing the moved rows again, with no scroll region. One synchronized update and one write per frame, at most 2,400 frames | an agent's fullscreen mode: many tiny positioned writes and a burst of rewritten rows every few frames; frames per second; depends on the window size |
| `diffscroll` | paging through a colored `git diff` in a pager: alternate screen, then page after page, each a full redraw (home, each row erased and drawn, hunk headers cyan, `+` green, `-` red) with a reverse-video status line | `git diff`, `less` and `delta`; depends on the window size |
| `editstorm` | an editor at work: a scroll region for the text, insert and delete lines (IL, DL), insert, delete and erase characters (ICH, DCH, ECH), syntax-colored repaints, cursor moves, a status line and a command line updated often, cursor shape changes (DECSCUSR) | vim, helix and nano editing a file; depends on the window size |

**Stress**: one thing at a time, pushed to the limit.

| Command | Workload |
|---|---|
| `logflood` | plain log lines, like `cat` of a big file |
| `glyphstorm` | CJK, emoji, combining marks, box drawing |
| `colorflood` | every character with its own 24-bit fg and bg |
| `scrollstorm` | lines pushed through a DECSTBM region with reverse index, like vim or tmux |
| `cursorstorm` | random cursor moves with short colored writes, like a full-screen TUI |
| `longlines` | lines of 5–50 thousand characters that wrap |
| `stylestorm` | every SGR attribute, densely mixed: bold, dim, italic, underline styles `4:1`–`4:5`, colored underlines `58:2::r:g:b`, strikethrough, overline, reverse, blink, conceal, 256-color and truecolor, resets. A terminal that doesn't draw a style still parses it, so the numbers stay comparable |
| `linkstorm` | OSC 8 hyperlinks on short spans (file names, compiler locations, web links), thousands of distinct URLs |
| `framerate` | full-screen redraws in synchronized output, in frames per second |
| `screenswap` | opening and closing a full-screen program over and over: leave the alternate screen, enter it, paint a screen; swaps per second |
| `roundtrip` | 1,000 cursor-report round trips on an idle terminal: p50, p90, p99 |

`ttystorm bench --only realistic` runs the first group and `--only stress` the second (`all` is every
test); names and groups can be mixed: `--only realistic,roundtrip`.

Options: `--plain` (no status line and no result screen, the results as plain text), `--size <MB>`
per test (default 16), `--runs <n>`, `--save <file.json>`, `--label <name>`, and for `ttystorm bench`
`--only logflood,roundtrip`. `--details` adds a diagnostics section per test after the results and
`--trace <file>` writes every write as JSON Lines (both below). `compare` warns when the files differ in
terminal size or machine: `agentredraw`, `agentscreen`, `diffscroll`, `editstorm`, `scrollstorm`,
`cursorstorm`, `colorflood`, `screenswap` and `framerate` depend on the size.
`ttystorm compare --markdown a.json b.json …` prints a markdown table instead of the screen: a row per
test, a column per terminal, the best of each row in bold, the overall ratio to the first file last, and a
note above when machines or sizes differ.

### Why a terminal is fast or slow

A number says how fast; the diagnostics say why. Every run is timed write by write, and the
terminal's own process is watched (best effort), so a result comes with the evidence to act on.
`--details` prints it as words, after the results screen:

```
logflood  412.0 MB/s  (run 2 of 3 shown, the median)
  payload: 16,000,512 bytes in 245 writes (avg 65306 bytes): 100% text, 0% escape sequences
  time: 0.039 s = writing 0.031 s (79%) + waiting for the answer after the last byte 0.006 s (15%) + other 0.002 s
  writes: 245 calls; each took p50 80 us, p99 4210 us, max 6.1 ms
  stalls: none over 20 ms
  timeline: ▇█  (MB/s per 100 ms, peak 430, 2 buckets)
  terminal: 0.09 s of CPU = 230% of wall time, 0.006 s per MB, memory +4 MB
  findings:
    - no stalls and no parse tail: the terminal kept up with the writer
```

What it measures, and how to read it:

- **Writing against waiting.** The benchmark writes through a pty. A `write` returns when the terminal has read
  enough to make room, so time blocked in `write` is time the terminal was not reading (**read-bound**).
  The time from the last byte to the terminal's answer (the cursor report) is the **parse tail**: input
  it had accepted but not yet parsed (**parse-bound** when it is most of the run). With tiny writes
  (`agentstream`) the time in `write` is mostly the cost of each call, not a full buffer.
- **Stalls.** A write that blocked for more than 20 ms, with when in the run it began. A few long ones
  at regular intervals are periodic pauses (a render, garbage collection, throttling).
- **Timeline.** MB/s the terminal accepted in each 100 ms. Dips are hiccups; a rising line is a warm-up.
- **The terminal process.** ttystorm walks up its parent processes to the first one without a controlling
  terminal (the terminal emulator; under `tmux`, `screen` or `ssh` that is named instead, and the
  numbers describe it) and reads its CPU time and memory with `ps` (or `/proc`) around each run. CPU
  seconds per MB is the efficiency; CPU over wall time well below 100% while the writer waits means
  the terminal is waiting (render gating, vsync), not computing; above 100% means several threads.
  Memory growth is what the output costs in scrollback and caches.
- **Payload composition.** Counted from the bytes: text against escape sequences, SGR, cursor moves,
  erases, scroll and edit sequences, OSC, DCS, mode sets, lines, characters and wide characters,
  writes. A terminal that is slow at `colorflood` but not at `logflood` is slow at SGR, not at bytes.

**For a coding agent:** give it the `--save` JSON and the `--trace` file and ask what to optimize in the
terminal. The fields:

`--save` file, top level: `format` (`ttystorm`), `format_version` (1), `tool_version`, `label`,
`terminal`, `cols`, `rows`, `machine`, `date`, `size_mb`, `runs`, `diag_version` (1; it changes when a
diagnostic field changes meaning), `terminal_process` (`{pid, command, note}`, or `null` when not found;
`note` is `tmux`, `screen` or `ssh` when the process found is in front of the terminal), `tests`.

Each test: `name`, `unit` (`MB/s`, `fps`, `µs`, `per_s`), `median`, `min`, `max`, `secs` (the median run),
`bytes`, `timeout`, `extra` (`mbps` and `frames` or `swaps` for `framerate`, `agentredraw`, `agentscreen` and
`screenswap`; `writes` and `writes_per_s` for `agentstream`, `agentredraw` and `agentscreen`; `p90`, `p99`
and `samples` for `roundtrip`) and `diag` (`null` for `roundtrip` and for files saved by older versions):

| `diag` field | Meaning |
|---|---|
| `composition` | `bytes`, `text_bytes`, `escape_bytes`, `sgr`, `cursor`, `erase`, `scroll` (scroll regions, IL/DL, ICH/DCH, SU/SD), `osc`, `dcs`, `modes`, `other`, `lines`, `chars`, `wide`, `writes` |
| `median_run` | index (from 0) of the run closest to the median, in `runs` |
| `runs[].secs` | the run's wall time until the terminal answered |
| `runs[].write_blocked_s` | seconds spent inside `write` calls |
| `runs[].dsr_wait_s` | seconds from the last write to the answer: the parse tail |
| `runs[].writes` | number of writes |
| `runs[].write_us` | `p50`, `p99`, `max` of how long a write took, in µs |
| `runs[].stalls` | `over_ms` (20), `count`, `total_s`, `longest` (up to 8 of `{at_s, ms}`, by time into the run) |
| `runs[].timeline` | `bucket_ms` (100) and `mb_per_s`, one value per bucket, the last one partial |
| `runs[].terminal` | `cpu_s`, `rss_start_kb`, `rss_end_kb` of the terminal process during the run; `null` when not measured |

The `--trace` file is JSON Lines, one event per write of every measured run (not the warm-up):

```json
{"test":"logflood","run":1,"event":"write","t_ms":12.345,"bytes":65536,"write_ms":0.412,"cum_bytes":131072}
{"test":"logflood","run":1,"event":"dsr","t_ms":40.1,"wait_ms":6.2,"cum_bytes":16000512}
```

`t_ms` is when the write returned, counted from the start of the run; `write_ms` is how long it took;
`cum_bytes` the bytes written so far. The last line of a run is the answer: `t_ms` is the run's total and
`wait_ms` the parse tail. Plot `cum_bytes` against `t_ms` to see the terminal's pace, or `write_ms` to
see the stalls. A 16 MB test makes a few hundred lines per run, but `agentstream` makes one for every
tiny write (hundreds of thousands): trace it alone with `--runs 1`.

Before 1.0, when it lived in Wormhole's repository, ttystorm was called termbench and the benchmarks
`ascii`, `unicode`, `truecolor`, `scroll-region`, `cursor`, `long-lines`, `frames` and `latency`.
They send the same bytes under their new names, files saved then (format `termbench`) still
compare, and `--only` and `--kind` still take the old names.

`ttystorm flood [--kind logflood|colorflood|glyphstorm|cursorstorm|…] [--seconds n]` writes one of these
workloads over and over, as fast as the terminal reads, until the time is up or `q` is pressed. It
measures nothing itself: it is the load for watching a terminal from outside, as Wormhole's
`scripts/perf.sh` does (draw time per frame, main-thread busy share and throughput against a
baseline).

What it doesn't measure: keypress-to-pixel latency (that needs a camera or a tool like
`typometer`), and frames actually drawn on screen.

## Feature check

`ttyprobe` asks the terminal what it supports and measures how it draws text, so terminals can be
compared on more than speed. Like every command here it is also `ttystorm ttyprobe`.

```sh
ttyprobe                                 # the result screen, then a sample page to look at
ttyprobe --details                       # + every query sent and received, and what it means
ttyprobe --save wormhole.json --label Wormhole
ttystorm compare wormhole.json ghostty.json kitty.json           # one matrix, differences marked
ttystorm compare --markdown wormhole.json ghostty.json > terminals.md
```

Options: `--plain` (the results as plain text, no colors, no sample page; the queries are still
sent), `--save <file.json>`, `--label <name>` (default `$TERM_PROGRAM`), `--details`, `--raw [file]`
and `--no-sample`.

### What it asks

Each row of the result screen is one question, and the terminal's own answer:

- **Identity**: XTVERSION (the terminal's name and version), the DA1 and DA2 device attributes, and
  `$TERM_PROGRAM`, `$TERM`, `$COLORTERM`, and whether a multiplexer (tmux, screen, zellij) is in the way.
- **Modes**, by DECRQM: synchronized output (2026), grapheme clustering (2027), bracketed paste, focus
  events, SGR mouse and SGR-pixels mouse, the alternate screen, color-scheme updates (2031), autowrap,
  cursor visibility and blink. Also the Kitty keyboard protocol's flags and the cursor style.
- **Graphics**: the Kitty graphics protocol (a one-pixel test image, `a=q`), Sixel (DA1 attribute 4) and
  iTerm2's inline images (its cell size report).
- **Colors**: foreground, background and palette color 1 as the terminal reports them (OSC 10, 11, 4),
  whether the background is dark, and truecolor: it sets a known 24-bit color and asks the terminal
  to report its graphic rendition (DECRQSS) to see if the color is still in it.
- **Sizes**: in cells, in window pixels and in cell pixels (`CSI 18/14/16 t`), and what the tty says.

A feature that the terminal answers (a ✓ or ✗) is known; `?` means it answered nothing and nothing is known.

### The DA1 trick

Every query is followed by DA1 (`CSI c`, "what are you"). A terminal answers its queries in
order, and practically every terminal answers DA1. So when the DA1 reply arrives *without* the
query's own reply, the answer is "not supported", known in microseconds, with no timeout to wait
out. Only a terminal that answers nothing at all costs time: the first query waits 1 s, and when
that gets no DA1 reply either, the rest is not asked and everything reads `?`.

### Why widths matter

How many cells a character takes is not settled. Unicode says an emoji with a skin tone or a family
joined by zero-width joiners is one character and two cells wide. A terminal that counts code points
draws the family as six cells, and the program running in it, which counted by the rules, puts the
cursor and the borders in the wrong place. The only way to learn what a terminal does is to ask where
the cursor is: `ttyprobe` goes to column 1, writes the text, asks for the cursor position (DSR 6) and
takes the distance. Twenty cases (ASCII, combining marks, CJK, Hangul jamo, emoji with VS16, skin tones,
ZWJ sequences, flags, keycaps, arrows, box drawing, a lone ZWJ) are compared with the width Unicode
15.1/16 gives their grapheme cluster. The Devanagari conjunct is disputed (fonts, Unicode and
terminals disagree), so it is shown but not scored. If the terminal has mode 2027 (grapheme
clustering) the cases are measured twice, as the terminal is and with the mode set, because that is the
difference programs have to know about; the mode is put back after.

The score reads "17 of 19 match".

### What it can't know

Some things cannot be asked, so they are not rows; the sample page printed after the results shows them
for eyes and screenshots: underline styles (`4:1` to `4:5`) and colored underlines, italic, bold, dim,
strikethrough, overline, blink, reverse, an OSC 8 hyperlink, box drawing, block elements and braille on
a ruler, powerline glyphs, and a grid of emoji and CJK where a misdrawn width shifts the bars. Clipboard
access (OSC 52) is also not asked: reading the clipboard is a privacy question and writing it changes
the user's clipboard. A ✗ on a mode means the terminal did not announce it, not that it never works:
a terminal can support a mode and still not answer DECRQM for it. Inside tmux or screen the answers
come from the multiplexer, not the terminal. Replies come from the terminal's parser, not its
renderer: a terminal can report a width and still draw something else with its font.

### Debugging and handing it to an agent

Every exchange is recorded: what was sent, the raw reply (escaped, `\x1b` is ESC), how long it took and
how the result was decided. `--details` prints them after the results, one line per query:

```
synchronized output (2026) | sent \x1b[?2026$p\x1b[c | received \x1b[?2026;2$y\x1b[?64;1;4c | 37 µs | reply
focus events (1004) | sent \x1b[?1004$p\x1b[c | received \x1b[?64;1;4c | 32 µs | sentinel
```

The width cases add the cursor reports before and after the text, so a width can be traced to the
bytes it came from. Anything unexpected is a line below with a `!`: replies that came out of order or
that nobody asked for, replies that don't have the expected shape, bytes already waiting when the
query went out. After the transcript comes **FINDINGS**, in plain words: "synchronized output (2026)
not reported: the terminal may still support it without answering DECRQM", "ZWJ sequences drawn 6
wide: grapheme clustering is off; enable mode 2027", "no reply at all within 1 s: probably not a real
terminal, or behind a multiplexer that swallows queries, try outside tmux".

How a result was decided (`decided`):

| Value | Meaning |
|---|---|
| `reply` | the terminal's reply arrived and was read |
| `sentinel` | DA1 came back first, without an answer: not supported |
| `timeout` | neither the answer nor DA1 came in time |
| `unparsed` | something came back, but not in the shape the query expects |
| `skipped` | not asked: an earlier query had already got no DA1 reply |
| `environment` | read from the environment (`$TERM`, the tty size) |
| `derived` | worked out from another row (Sixel from DA1, dark background from OSC 11) |

`--raw` also dumps every byte read from the terminal while probing, one chunk per line with the
time it arrived, to stderr, or to a file with `--raw probe-raw.txt`. It is for debugging `ttyprobe`
itself (a reply that was split, or that no query accounts for).

**Give the `--save` JSON to your coding agent** to find out why something reads as unsupported: it has
the transcripts and the findings, and they were written to be read without running anything again.

The file (`"format": "ttystorm-probe"`):

| Field | Meaning |
|---|---|
| `format`, `format_version`, `tool_version` | `ttystorm-probe`, the file layout's version, `ttystorm`'s version |
| `label`, `terminal`, `cols`, `rows`, `date` | the name given with `--label`, `$TERM_PROGRAM` and its version, the size in cells, the day |
| `machine` | `cpu`, `model`, `os`, `memory`, `cores`, as in benchmark files |
| `score_as_is`, `score_2027` | `{"matched": 17, "counted": 19}` over the cases that are not disputed; `null` when not measured |
| `findings` | the findings, plain sentences |
| `events` | unexpected things that belong to no row (a first query with no DA1 reply, giving up) |
| `features[]` | one per row, see below |
| `widths[]` | one per width case, see below |

A `features[]` entry: `section` (Identity, Modes, Graphics, Colors, Sizes), `name`, `status` (`yes`, `no`,
`value`, `unknown`), `value` (for `value` rows: `"#1e1e2e"`, `"WezTerm 2024"`), `note` (a detail beside
the mark: a mode's state `on`/`off`, `flags 31`, `not reported`), `how` (the short form of the query)
and its transcript: `decided`, `sent`, `received`, `micros` (from the request to the reply that decided it),
`notes` (the unexpected).

A `widths[]` entry: `name`, `codepoints` (`1F468 200D 1F469`), `expected` (cells), `disputed`, `measured` and
`measured_2027` (cells, or `null`), and `as_is` and `with_2027` (`null` when the terminal has no mode 2027),
each `{"width", "dsr_before", "dsr_after", "decided", "sent", "received", "micros", "notes"}`.
`dsr_before` and `dsr_after` are the raw cursor reports, such as `\x1b[2;1R` and `\x1b[2;3R`; the width is
the difference of their columns.

### Comparing terminals

Run `ttyprobe --save <terminal>.json` in each terminal (outside tmux; the same window size is not
needed), then `ttystorm compare a.json b.json …`. For probe files the result is a matrix: a column per
terminal, a row per feature and per width case, ✓ ✗ ? or the value, widths green when they match
Unicode and red when they don't, and `≠` marking the rows where the terminals differ. Identity and size
rows are expected to differ and are never marked. `--markdown` prints the same as tables for an issue or
a README. Benchmark files and probe files can't be mixed in one `compare`; it names which is which.

## Recording for social media

- **Same conditions for every terminal**: the same window size (the overlay, `h`, shows cols×rows), font
  size and machine, with nothing else busy. Quit other heavy apps; plug the laptop in.
- **Clean frame**: hide Wormhole's sidebar and panel, a dark theme; `--hud` only when the numbers are
  the point. macOS screen recording: ⇧⌘5, or `ffmpeg -f avfoundation` at 60 fps.
- **Loops**: `warptunnel --seconds 12` gives a clip with a clean start and end; the
  fire's word comes 2.5 s into every 12 s cycle.
- **Tearing vs not**: record `plasma --fps 0 --no-sync` next to `plasma --fps 0`.
- **Stress**: `--fps 0 --hud` shows how many frames and MB/s the terminal takes in;
  side by side with another terminal, it is the most direct comparison.
- **Numbers**: `ttystorm bench --save <name>.json` in each terminal, then
  `ttystorm compare a.json b.json …` for one screen with them all, best marked, ratios to the first
  file. The results screen paints its own background, so it looks the same in any theme.
