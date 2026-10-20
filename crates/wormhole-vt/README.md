# wormhole-vt

A terminal engine in Rust. Bytes from a program go in; the screen, the scrollback and the events the
program caused come out.

It is the engine of [Wormhole][repo], a macOS terminal for AI coding agents, and it is a library of its
own: MIT licensed, with no PTY, no window, no threads, no I/O and no `unsafe` code. It has two
dependencies, `bitflags` and `memchr`.

> **Status.** Version 0.1: the first release as a library. Until 1.0 the API may change between minor
> versions; the [changelog][changelog] lists every change, and `api.txt` in the repository is the whole
> public API as a diff-able list.

## Why

- **Written from the specifications.** The parser follows Paul Williams' DEC state machine, the
  sequences follow xterm's *Control Sequences*, the DEC manuals and ECMA-48, the extensions follow
  their published documents (kitty, iTerm2, OSC 8, synchronized output, Unicode). Each conformance
  case names the line of text it rests on.
- **Fast, and steady about it.** The parser finds runs of printable ASCII a word at a time. Stored
  cells are eight bytes. In the steady state (the same terminal, more output) the engine allocates
  nothing for most kinds of output; counting tests in the repository fail when that changes.
- **Safe against hostile output.** A terminal reads whatever a program prints, including a `cat` of a
  file from the internet. The parser is total, every string and table has a cap, replies are built from
  the engine's own state, and what is risky (the clipboard, window control, reading the screen back)
  is a request to the host or off until the host allows it. See [the security chapter][security].
- **Made to be embedded.** The host owns the PTY, the threads and the drawing. The engine tells it what
  changed, so a frame copies only the rows that did.

## Quick start

```rust
use wormhole_vt::{Color, Column, Event, Flags, Line, NamedColor, Point, Terminal};

let mut term = Terminal::new(80, 24);

// Bytes from the program, in any chunking: a sequence may be cut anywhere.
term.feed(b"\x1b[1;31mhello\x1b[0m\r\n");
term.feed(b"\x1b]2;my title\x07");
term.feed(b"\x1b[6n"); // the program asks where the cursor is

// What the program is waiting to read: write these bytes back to the PTY.
assert_eq!(term.take_replies(), b"\x1b[2;1R");

// What happened besides drawing.
let events: Vec<Event> = term.events().collect();
assert!(events.iter().any(|event| matches!(event, Event::Title)));
assert_eq!(term.title(), Some("my title"));

// What is on screen.
let cell = term.cell(Point::new(Line(0), Column(0))).expect("inside the screen");
assert_eq!(cell.c, 'h');
assert!(cell.flags.contains(Flags::BOLD));
assert_eq!(cell.fg, Color::Named(NamedColor::Red));
assert_eq!(term.row_text(Line(0), true), "hello");

// Everything in one place when you need more than the defaults.
let term = Terminal::builder().size(120, 40).scrollback(10_000).build();
assert_eq!((term.cols(), term.rows()), (120, 40));
```

`examples/quickstart.rs` is this, as a program you can run. The guide goes on from here, and in this
documentation its chapters are the modules of `guide`:

| Chapter | About |
|---|---|
| [concepts][concepts] | the model: the terminal as a state machine, cells, modes, replies, events, pins |
| [embedding][embedding] | the PTY loop, threads, resizing, scrollback, limits, policy, attach |
| [rendering][rendering] | damage, rows and cells, the cursor, selection, links, pictures, colours |
| [input][input] | keys, mouse, paste and focus as bytes for the program |
| [passthrough][passthrough] | your own OSC, DCS and APC strings, handed over whole |
| [security][security] | the threat model, the guarantees, what the host must still do |
| [conformance][conformance] | how the engine is tested, and how to check it yourself |
| [tools][tools] | debugging a stream, benchmarks, profiling, fuzzing |

## The crate

The types a host uses most are at the root: `Terminal` (made by `Terminal::new` or
`Terminal::builder`), `Event`, `Policy`, `Limits`, `Damage`, and the values they speak in (cells,
colours, positions, `Modes`). What a program may do beyond printing is a `Policy`, all off by default.
The groups that need a namespace are modules:

| Module | What is in it |
|---|---|
| `parser` | the DEC/ANSI state machine on its own, for tools that want the sequences and not a screen |
| `input` | keys (legacy, modifyOtherKeys, VT52 and the kitty protocol), mouse, paste and focus, as pure functions of an event and the modes |
| `passthrough` | sequences of the host's own, handed over whole as events |
| `event` | what a program's sequences ask of the host |
| `image` | pictures (kitty graphics, Sixel, iTerm2): uploads to decode, placements to draw |
| `window` | what XTWINOPS lets a program ask of the window, and where the window is |
| `repaint` | the VT bytes that rebuild one terminal's state in another |
| `unicode` | cell widths and grapheme cluster breaks from generated tables |
| `guide` | the chapters above |

There are no cargo features for users. `internals` exists for the repository's own tools and tests (it
opens the storage and a few checks); it is hidden from the documentation, unstable and outside the
semantic versioning promise. Do not enable it in an application; to run the crate's tests from a copy of
the package, use `cargo test --features internals`.

## What it does

| Area | Supported |
|---|---|
| Parser | DEC/ANSI state machine (VT500 diagram), UTF-8 with replacement characters per Unicode §3.9, C0 and C1 (7-bit and 8-bit), CSI with sub-parameters (`:`), OSC, DCS and APC strings streamed to the engine in pieces. A sequence may be cut at any byte. Usable alone, without a screen: `parser::Parser` with the `parser::Handler` trait |
| Conformance levels | VT100 to VT500 (DECSCL), DA1/DA2/DA3, DECRQM, DECRQSS, DECRQPSR/DECRSPS (cursor information, tab stops), DECRQTSR, XTGETTCAP, XTVERSION. VT52 mode (`CSI ? 2 l`) |
| Screen editing | Cursor movement, scrolling regions (DECSTBM) and left/right margins (DECSLRM), insert and delete of characters, lines and columns, tab stops, origin mode, delayed wrap, reverse wraparound (45, 1045), selective erase (DECSED, DECSEL, DECSCA), guarded areas (SPA, EPA), double-width and double-height lines (DECDWL, DECDHL, DECSWL) |
| Rectangular operations | DECCRA, DECFRA, DECERA, DECSERA, DECCARA, DECRARA, DECSACE |
| Character sets | G0 to G3: ASCII, UK, DEC special graphics, DEC Technical, DEC Supplemental, ISO Latin-1 supplement, the VT200 national replacement sets (DECNRCM), locking and single shifts |
| SGR | Bold, dim, italic, underline (single, double, curly, dotted, dashed: `4:0` to `4:5`, `21`), blink, inverse, hidden, strikethrough, overline; 16, 256 and true colours with `;` or `:`; underline colour (58, 59); XTPUSHSGR, XTPOPSGR, XTREPORTSGR |
| Colours | Palette and dynamic colour set, query and reset: OSC 4, 5, 10 to 19 and 104, 105, 110 to 119, with the host's colours as the answer to a query; XTPUSHCOLORS, XTPOPCOLORS, XTREPORTCOLORS |
| Strings | Title and icon name with the XTWINOPS stack (OSC 0, 1, 2; `CSI 22/23 t`), OSC 7 (folder), OSC 8 (hyperlinks), OSC 52 (clipboard, as host requests), OSC 133 (shell marks: prompt, command, output, exit code), OSC 9, OSC 9;4 (progress), OSC 99 and OSC 777 (notifications, as host requests), the program status protocol (OSC 7501), OSC 22 (pointer shape) and OSC 1337 `SetUserVar`, both as host events |
| Images | kitty graphics protocol (including Unicode placeholders), Sixel (decoded by the engine), iTerm2 inline images. The engine owns placement; the host decodes PNG/JPEG and draws |
| Unicode | Unicode 18 cell widths, extended grapheme clusters (UAX #29) with mode 2027 (on by default), emoji variation selectors, wide characters with spacer cells |
| Scrollback and reflow | History capped in bytes; soft-wrapped lines rewrap on resize with the cursor and pins carried along; the alternate screen is cropped or padded. Lines that leave the history can be handed to the host (`Event::HistoryDropped`, opt in) |
| Keyboard | kitty keyboard protocol (five flags, a stack per screen), xterm's modifyOtherKeys, application cursor and keypad modes, bracketed paste, focus reports, the VT52's keys |
| Mouse | Modes 9, 1000, 1002, 1003; encodings X10, UTF-8 (1005), URXVT (1015), SGR (1006), SGR with pixels (1016); the DEC locator (DECELR and the reports) |
| Output control | Synchronized output (mode 2026) as a flag the host reads; `ModesChanged` and `ColorsChanged` events instead of polling |
| Host reports | Light and dark (`CSI ? 996 n`, mode 2031), in-band resize notifications (mode 2048), pointer shapes (OSC 22) and iTerm2 user variables as events |
| Host side | Damage tracking and row copies into reusable buffers, selection (cell, block, word, line) and text extraction, pins and spans that follow text through scrolling and reflow, search support |
| Repaint writer | `repaint` serializes a terminal into VT bytes that rebuild it in another terminal, for attach and resync |
| Window | XTWINOPS reports always; operations (move, resize, maximise, full screen) and DECRQCRA only when `Policy` allows |

Not done, on purpose or for now: the printer (media copy, `CSI Ps i`, is recognised and ignored, and what
follows it is drawn), pages and panning, the VT500's Greek, Hebrew, Turkish and Cyrillic character sets,
ReGIS, soft fonts, user-defined keys, the status line, 8-bit GR characters (the input is UTF-8 only),
kitty's animation frames and its file and shared-memory image media, colour names and the `rgbi:`/`CIE…`
colour forms. The engine never reads files, runs programs or loads fonts.

## Conformance

The repository has 875 plain-text cases (`tests/conformance/*.vtt`), 752 of them quoting the published
text they follow and 123 marked as conventions (behaviour no manual spells out, such as the delayed wrap).
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

Three suites we did not write, all run against the engine alone, headless, with the expectations kept in
the repository (`scripts/suites-gate.sh`, `scripts/vttest-gate.sh`):

- **esctest2** (Thomas Dickey, ~560 tests against xterm's behaviour). 544 of 567 pass. Of the 23
  failures, 21 are Xcms colour forms (`rgbi:`, `CIEXYZ:` and the like) and two want a printer port in
  DA1. In the real app on 2026-10-15: 538 of 566.
- **ucs-detect** (Jeff Quast, how a terminal sizes Unicode). 15,012 of 15,012 sequences of its default run
  (the contested code points and graphemes), with mode 2027 on; in the app on 2026-10-15 too.
- **vttest** (Thomas Dickey, release 20251205). Of 455 screens, each judged by hand against what vttest
  says it should show: 387 pass, 5 fail and 56 are features the engine does not have. The five are by
  design: three need 8-bit GR characters (the engine reads UTF-8 only) and two compare a rectangle
  checksum with the VT420's formula where the engine answers xterm's.

Method, engines, dates and the full tables: [`tools/vt-compare/README.md`][compare] and
[`tools/vt-compare/results/2026-10-15-Mac17-8-Apple-M5-Pro/`][results] in the repository; the
[conformance chapter][conformance] says how to run each.

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
engine made no allocator calls on five of the nine corpora; `unicode-mix`, `osc8-links`, `payloads` and
`agent-session` still allocated.

This is one run on a laptop with other work going on (load average 3.89, 11.22, 11.83 at the start), so
it is a guide and not a verdict, and small differences prove nothing. The engines differ in what they
compute while they parse, and each runs behind an adapter we wrote. The engine has been tuned since that
run and the table has not been measured again. Rerun it yourself: `cargo run --release --bin vt-bench` in
`tools/vt-compare` (method in its README), or `vtbench` for the engine alone (see [the tools
chapter][tools]).

## Stability

0.x: the API may change between minor versions, and the [changelog][changelog] lists every change. Enums
and option structs are `#[non_exhaustive]`, so adding a variant or a field is not a breaking change.
`api.txt` is the public API as `cargo public-api` prints it; the repository's check
(`scripts/vt-api.sh`) fails when the API changes without the file, so a change always shows in review.

The minimum supported Rust version is **1.88**: the first release in which `let` chains (the 2024
edition's) and `slice::as_chunks` are stable, both of which the engine uses. It was found by running
`cargo check` on the crate with Rust 1.85, 1.86, 1.87 and 1.88: the first three fail, 1.88 builds.
Raising it is a minor version bump while the crate is 0.x.

## Security

Hostile output is the threat model. See [the security chapter][security], and [`SECURITY.md`][reporting]
for how to report a vulnerability.

## Licence

MIT. See `LICENSE`.

[repo]: https://github.com/wormhole-terminal/wormhole
[changelog]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/CHANGELOG.md
[reporting]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/SECURITY.md
[compare]: https://github.com/wormhole-terminal/wormhole/blob/main/tools/vt-compare/README.md
[results]: https://github.com/wormhole-terminal/wormhole/tree/main/tools/vt-compare/results/2026-10-15-Mac17-8-Apple-M5-Pro
[concepts]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/concepts.md
[embedding]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/embedding.md
[rendering]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/rendering.md
[input]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/input.md
[passthrough]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/passthrough.md
[security]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/security.md
[conformance]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/conformance.md
[tools]: https://github.com/wormhole-terminal/wormhole/blob/main/crates/wormhole-vt/docs/tools.md
