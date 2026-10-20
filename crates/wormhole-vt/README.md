<!--
API names assumed here (target API of engine-library.md; reconcile when phases 1-3 land):
  Terminal::new(cols, rows), Terminal::builder() -> Builder, Builder::{size, scrollback_lines, policy,
  cell_size, palette, word_separators, passthrough, build}, Terminal::feed(&[u8]),
  Terminal::take_replies() -> Vec<u8>, Terminal::events() (draining iterator), Event::{Title, Bell,
  ClipboardStore, ClipboardLoad, Notification, ..}, Terminal::resize(cols, rows),
  Terminal::cell(Point) -> Option<Cell>, Point/Line/Column/Cell at the crate root, Terminal::title() -> Option<&str>, Terminal::cursor(),
  Terminal::take_damage(&mut Damage), Policy { window_ops, title_report, screen_checksum }, input::*.
-->

# wormhole-vt

A terminal engine in Rust. Bytes from a program go in; the screen, the scrollback and the events the
program caused come out.

It is the engine of [Wormhole](https://github.com/wormhole-terminal/wormhole), a macOS terminal for
AI coding agents, and it is a library of its own: MIT licensed, with no PTY, no window, no threads, no
I/O and no `unsafe` code. It has two dependencies, `bitflags` and `memchr`.

> **Status.** This crate has not been published to crates.io yet. The API below is the 0.1 API; some of
> it is still being built (see `CHANGELOG.md`). Until 0.1.0 is released, treat every name as provisional.

## Why

- **Written from the specifications.** The parser follows Paul Williams' DEC state machine, the
  sequences follow xterm's *Control Sequences*, the DEC manuals and ECMA-48, the extensions follow
  their published documents (kitty, iTerm2, OSC 8, synchronized output, Unicode). Each conformance
  case names the line of text it rests on.
- **Fast, and steady about it.** The parser finds runs of printable ASCII a word at a time. Cells are
  eight bytes. In the steady state (the same terminal, more output) the engine allocates nothing for
  most kinds of output; counting tests in the repository fail when that changes.
- **Safe against hostile output.** A terminal reads whatever a program prints, including a `cat` of a
  file from the internet. The parser is total, every string and table has a cap, replies are built from
  the engine's own state, and what is risky (the clipboard, window control, reading the screen back)
  is a request to the host or off until the host allows it. See `docs/security.md`.
- **Made to be embedded.** The host owns the PTY, the threads and the drawing. The engine tells it what
  changed, so a frame copies only the rows that did.

## Quick start

```rust,ignore
use wormhole_vt::{Column, Line, Point, Terminal};

let mut term = Terminal::new(80, 24);

// Bytes from the program, in any chunking: a sequence may be cut anywhere.
term.feed(b"\x1b[1;31mhello\x1b[0m\r\n");
term.feed(b"\x1b]2;my title\x07");
term.feed(b"\x1b[6n"); // the program asks where the cursor is

// What the program is waiting to read: write these bytes back to the PTY.
let replies = term.take_replies();
assert_eq!(replies, b"\x1b[2;1R");

// What happened besides drawing.
for event in term.events() {
    println!("{event:?}"); // Event::Title, ...
}
assert_eq!(term.title(), Some("my title"));

// What is on screen.
let cell = term.cell(Point::new(Line(0), Column(0))).expect("inside the screen");
assert_eq!(cell.c, 'h');

// Everything in one place when you need more than the defaults.
let term = Terminal::builder()
    .size(120, 40)
    .scrollback_lines(10_000)
    .build();
```

The chapters under `docs/` go on from here: `concepts.md` (the model), `embedding.md` (the PTY loop,
threads, resizing), `rendering.md` (damage, cursor, selection, images), `input.md` (keys, mouse, paste,
focus), `extensions.md` (your own sequences), `security.md` (the threat model), `conformance.md`
(how the engine is tested) and `tools.md` (debugging a stream, benchmarks, profiling, fuzzing).

## What it does

| Area | Supported |
|---|---|
| Parser | DEC/ANSI state machine (VT500 diagram), UTF-8 with replacement characters per Unicode §3.9, C0 and C1 (7-bit and 8-bit), CSI with sub-parameters (`:`), OSC, DCS and APC strings streamed to the engine in pieces. A sequence may be cut at any byte. Usable alone, without a screen: `parser::Parser` with the `parser::Handler` trait |
| Conformance levels | VT100 to VT500 (DECSCL), DA1/DA2/DA3 answered as that family does, DECRQM, DECRQSS, DECRQPSR/DECRSPS (cursor information, tab stops), DECRQTSR, XTGETTCAP, XTVERSION |
| Screen editing | Cursor movement, scrolling regions (DECSTBM) and left/right margins (DECSLRM), insert and delete of characters, lines and columns, tab stops, origin mode, delayed wrap, reverse wraparound (45, 1045), selective erase (DECSED, DECSEL, DECSCA), guarded areas (SPA, EPA) |
| Rectangular operations | DECCRA, DECFRA, DECERA, DECSERA, DECCARA, DECRARA, DECSACE |
| Character sets | G0 to G3: ASCII, UK, DEC special graphics, DEC Technical, DEC Supplemental, ISO Latin-1 supplement, the VT200 national replacement sets (DECNRCM), locking and single shifts |
| SGR | Bold, dim, italic, underline (single, double, curly, dotted, dashed: `4:0` to `4:5`, `21`), blink, inverse, hidden, strikethrough, overline; 16, 256 and true colours with `;` or `:`; underline colour (58, 59) |
| Colours | Palette and dynamic colour set, query and reset: OSC 4, 5, 10 to 19 and 104, 105, 110 to 119, with the host's colours as the answer to a query |
| Strings | Title and icon name with the XTWINOPS stack (OSC 0, 1, 2; `CSI 22/23 t`), OSC 7 (folder), OSC 8 (hyperlinks), OSC 52 (clipboard, as host requests), OSC 133 (shell marks: prompt, command, output, exit code), OSC 9, OSC 9;4 (progress), OSC 99 and OSC 777 (notifications, as host requests) |
| Images | kitty graphics protocol (including Unicode placeholders), Sixel (decoded by the engine), iTerm2 inline images. The engine owns placement; the host decodes PNG/JPEG and draws |
| Unicode | Unicode 18 cell widths, extended grapheme clusters (UAX #29) with mode 2027 (on by default), emoji variation selectors, wide characters with spacer cells |
| Scrollback and reflow | History capped in bytes; soft-wrapped lines rewrap on resize with the cursor and pins carried along; the alternate screen is cropped or padded |
| Keyboard | kitty keyboard protocol (five flags, a stack per screen), application cursor and keypad modes, bracketed paste, focus reports |
| Mouse | Modes 1000, 1002, 1003; encodings X10, UTF-8 (1005), SGR (1006); the DEC locator (DECELR and the reports) |
| Output control | Synchronized output (mode 2026) as a flag the host reads |
| Host side | Damage tracking and row copies into reusable buffers, selection (cell, block, word, line) and text extraction, pins and spans that follow text through scrolling and reflow, search support |
| Repaint writer | `repaint` serializes a terminal into VT bytes that rebuild it in another terminal, for attach and resync |
| Window | XTWINOPS reports always; operations (move, resize, maximise, full screen) and DECRQCRA only when `Policy` allows |

Not done: printing (Media Copy), pages and panning, the VT500's Greek, Hebrew, Turkish and Cyrillic
character sets, ReGIS, kitty's animation frames and its file and shared-memory image media, colour
names and the `rgbi:`/`CIE…` colour forms. The engine never reads files, runs programs or loads fonts.

## Conformance

The repository has 765 plain-text cases (`tests/conformance/*.vtt`), 660 of them quoting the published
text they follow and 105 marked as conventions (behaviour no manual spells out, such as the delayed wrap).
Every case runs twice, once with the bytes fed whole and once a byte at a time.

The kept comparison run (2026-10-15, Apple M5 Pro, macOS 27.0.1) used the 758 cases that existed then. It
ran them against five engines, each as a black box behind its public API:

| Engine | Cases passed (of 758) |
|---|--:|
| wormhole-vt | 758 |
| libghostty-vt (a commit of main) | 558 |
| wezterm-term (a commit of main) | 484 |
| alacritty_terminal 0.26.0 | 447 |
| vt100 0.16.2 | 313 of the 581 it can be asked |

Read this with care: the cases are ours, written from the same specifications the engine follows, so our
engine passing all of them says the suite and the engine agree, not that the engine is complete. A
failure of another engine is a difference from our reading of the text. Where we only follow a
convention, the matrix says so, and 20 of the 103 conventions in that run fail on most engines.

Two suites we did not write:

- **esctest2** (Thomas Dickey, ~560 tests against xterm's behaviour). The engine alone, headless, passes
  544 of 567. Of the 23 failures, 21 are Xcms colour forms (`rgbi:`, `CIEXYZ:` and the like) and two
  want a printer port in DA1. In the real app on 2026-10-15: 538 of 566.
- **ucs-detect** (Jeff Quast, how a terminal sizes Unicode). 15,012 of 15,012 sequences of its default run
  (the contested code points and graphemes), with mode 2027 on.

Method, engines, dates and the full tables: `tools/vt-compare/README.md` and
`tools/vt-compare/results/2026-10-15-Mac17-8-Apple-M5-Pro/` in the repository.

## Speed

Throughput of the engine alone (bytes in, screen state updated; no drawing). Release build, Mac17,8
(Apple M5 Pro), 2026-10-15. Each corpus is 8 MiB of a deterministic stream, fed in 64 KiB pieces to a
fresh terminal; the fastest of 5 passes counts. MB/s, higher is better.

| corpus | wormhole-vt | alacritty_terminal | libghostty-vt | wezterm-term | vt100 |
|---|--:|--:|--:|--:|--:|
| ascii-flood | 2764 | 233 | 1800 | 67 | 257 |
| log-lines | 1528 | 248 | 1207 | 66 | 241 |
| sgr-churn | 379 | 232 | 165 | 49 | 253 |
| unicode-mix | 330 | 207 | 414 | 70 | 212 |
| tui-redraw | 754 | 244 | 465 | 42 | 210 |
| scroll-region | 809 | 212 | 648 | 57 | 220 |
| osc8-links | 469 | 191 | 27 | 70 | 299 |
| payloads | 569 | 422 | 1008 | 291 | 389 |
| agent-session | 534 | 320 | 231 | 71 | 353 |

libghostty-vt is faster on `unicode-mix` and `payloads`. On a second pass over the same terminal, the
engine makes no allocator calls on five of the nine corpora; `osc8-links` (each link owns its target),
`payloads` and `agent-session` still allocate.

This is one run on a laptop with other work going on (load average 3.89, 11.22, 11.83 at the start), so
it is a guide and not a verdict, and small differences prove nothing. The engines differ in what they
compute while they parse, and each runs behind an adapter we wrote. Rerun it yourself:
`cargo run --release --bin vt-bench` in `tools/vt-compare` (method in its README).

## Stability

0.x: the API may change between minor versions, and `CHANGELOG.md` lists every change. Enums and
option structs are `#[non_exhaustive]`, so adding a variant or a field is not a breaking change. The
minimum supported Rust version will be stated here once it is tested in CI; the crate uses the 2024
edition, which needs Rust 1.85 or newer.

## Security

Hostile output is the threat model. See `docs/security.md`, and `SECURITY.md` for how to report a
vulnerability.

## Licence

MIT. See `LICENSE`.
