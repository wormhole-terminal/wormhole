# vt-compare

Wormhole's engine tools (`crates/wormhole-vt-bench`) pointed at other terminals' engines, so the
new engine is tested and timed against more than one other:

| tool | what | run |
|------|------|-----|
| `vt-matrix` | every `.vtt` conformance case (`crates/wormhole-vt/tests/conformance/`) on every engine: passes per area, the cases each engine fails, where engines disagree, and which of our `convention` cases most engines fail | `cargo run --release --bin vt-matrix` (`--markdown`, `--json`, `--engine`, `--filter`, `--no-lists`) |
| `vt-bench` | the vtbench corpora on every engine: MB/s and allocator calls per MB, the same method as `vtbench` | `cargo run --release --bin vt-bench` (`--markdown`, `--json`, `--save <file>`, `--engine`, `--corpus`, `--runs`, `--size`) |
| parity tests | Wormhole's engine against alacritty on random streams (cleaned of what alacritty gets wrong, `tests/parity/sanitize.rs`; the delayed wrap mended, `fixed.rs`; the few differences that are nobody's mistake left out, `compare.rs`), whole and cut; `explore` lists what differs, `show` prints both screens for one stream, `recordings` compares real sessions. `tests/selection.rs` does the same for selecting and copying text | `cargo test --test parity` (`-- explore --ignored --nocapture`, see the file's header for the `PARITY_*` variables), `cargo test --test selection` |
| `parse_bench` | the parser's throughput against `vte` (alacritty's parser) on deterministic inputs | `cargo run --release --example parse_bench` |

## Suites we did not write

The matrix above is ours, and our engine passes all of it. `scripts/suite-compare.sh` runs two outside
suites in the real apps (Wormhole, Ghostty, Alacritty, kitty, WezTerm, iTerm2, Terminal.app), one app and
one suite at a time, and makes a markdown table per suite in `results/<date>-<machine>/suites/`:

| suite | what | table |
|-------|------|-------|
| [esctest2](https://github.com/ThomasDickey/esctest2) (GPL-2.0) | ~560 automated tests of escape sequences against xterm's behaviour; reads the screen back with DECRQCRA checksums | passes per terminal and per area (`esctest.md`), every test (`esctest-tests.md`) |
| [ucs-detect](https://github.com/jquast/ucs-detect) (MIT) | how a terminal sizes Unicode: wide, narrow, ZWJ emoji, VS15/VS16, flags, languages, by cursor position reports | share sized right per category, next to the author's published results for the same terminal (`ucs-detect.md`) |

A third suite runs headless against Wormhole's engine only, see [vttest](#vttest) below:
[vttest](https://invisible-island.net/vttest/) (MIT-X11), Thomas Dickey's menu driven VT100/VT220/VT420/VT520 and xterm test program.

`./fetch-suites.sh` fetches both at the commits in `suites.pin` into the cache (not into this repository)
and makes a Python venv for them (it also builds vttest, below); `scripts/suite-compare.sh` calls it. `suites/` holds our side:
`esctest_run.py` runs inside the terminal (a probe of the reports esctest2 needs, then the tests, one
JSON line each) and `report.py` makes the tables. Start with `scripts/suite-compare.sh --probe-only`
(20 s per app) to see which apps answer what esctest2 needs.

`ucs-headless.py` runs ucs-detect against Wormhole's engine alone, with no window or app: a
pseudo-terminal whose other end is `ucs-host` (`crates/wormhole-vt-bench`), which feeds the engine and
writes its cursor position reports back. A run takes a second, so it is the way to work on widths
(`--clusters on|off` sets mode 2027 first, `-- --all` tests every code point instead of the contested ones).

esctest2 is judged against the behaviour it describes, not against xterm's known-bug annotations
(a terminal that does better than xterm is not failing), with "empty" and "blank" cells counted as one.
Wormhole answers DECRQCRA only with `allow_screen_checksum` on (a program could otherwise read the screen back cell by cell), so the scripts start it with that key set in its throwaway config (and `allow_window_ops` and `allow_title_report`, which the window-operation and title-report tests need). A terminal that does not answer DECRQCRA cannot be asked what is on screen: a test that reaches a screen
check is `unmeasurable` for it and left out of its count, and the table says how many tests that is
(about 215 of 559 look at the screen), so the comparison between apps is also made on the tests
every measurable app ran.

End to end, in the real apps (Ghostty, kitty, WezTerm, iTerm2, Terminal.app, Alacritty, Wormhole):
`scripts/terminal-compare.sh`. kitty, iTerm2 and Terminal.app have no engine one can embed, so they
are only in that one.

It is its own Cargo workspace, like `tools/bench`: its dependencies (WezTerm's model alone is
several dozen crates) stay out of the app's `Cargo.lock`, lints and `cargo deny`, and
`scripts/check.sh` skips it. It depends on `wormhole-vt-bench` by path, so an engine registered
there (Wormhole's own) is in the matrix and the table without a change here. alacritty_terminal is
here and nowhere else: the app ran on it until its own engine replaced it. Checks: `cargo fmt`,
`cargo clippy --all-targets -- -D warnings` and `cargo test`, all in this folder.

## vttest

[vttest](https://invisible-island.net/vttest/) is Thomas Dickey's test program for VT100 terminals and their successors. It is menu
driven and draws one screen per test with a description of how that screen should look (the menus: cursor movements, screen
features, character sets, double-sized characters, keyboard, terminal reports, VT52 mode, insert and delete, known VT100
bugs, reset, and the non-VT100 menu with VT220 to VT520 features, ISO 6429 colours and xterm's extensions). Its licence is the
MIT-X11 permission notice (`COPYING` in the tarball): it is only run here, downloaded into the suites cache and never linked or
committed. `suites.pin` names the release (20251205) and the SHA-256 of its tarball; `fetch-suites.sh` checks the sum, runs
`./configure && make` in the cache, and builds it with `suites/vttest-fast.h`, which shortens vttest's pauses between a request
and the answer (a tenth of a second, three times for each report) to a quarter: a terminal that is a function call away answers at
once, and the pauses only made the run take minutes. It needs `curl`, `make` and a C compiler; without them or the network the
step is skipped with a warning.

`vttest-host` (`crates/wormhole-vt-bench`) is the terminal at the other end of a pseudo-terminal (`suites/pty_relay.py` keeps
it): the engine is fed what vttest writes, its replies go back at once, DECCOLM and the window operations are played by the same
fake window as `esctest-host`'s (`window_host`), and the menus are walked from what vttest prints (the prompts and the entries
on the screen), so an entry added to vttest is walked without a change. At every `Push <RETURN>` it writes the engine's screen
to `suites/vttest/<menu>/<nn>.txt`: the rows as text (a character set maps characters before they reach the screen, so the
line-drawing glyphs are text already), the attributes and colours of the cells as runs, the size of every row that is not normal (double width, double height), the modes that differ from vttest's own
start, the cursor with the style a program chose, and the path of choices that led there. `--only 11.3.7` walks one entry on its
own, `--trace` prints the whole conversation, `--timing` says how long each part took. The non-VT100 menu is walked in parts, each
in a vttest of its own, a few at once, so a whole run takes about ten seconds.

`scripts/vttest-gate.sh` (run by `scripts/suites-gate.sh` and so by `scripts/check.sh` where the other suites run) walks
vttest again and fails when a screen differs from its committed one, printing the diff (a difference is walked a second time
before it counts, so a machine too busy to answer vttest in time does not fail it). `--update` rewrites the screens after a
change on purpose; update each menu's `review.md` with it.

**What the screens are.** The committed screens are the engine's behaviour today. Each menu's folder has a `review.md`: every
screen judged against what vttest says it should show (and the VT100/VT220/VT510 manuals and xterm where that is not enough),
`pass`, `fail: <why>` or `unsupported: <feature>`. Of 455 screens, 387 pass, 5 fail, 56 are unsupported features, 6 are prompts
(a screen waiting for the key the host then presses) and one is not exercised. By menu (screens: pass, fail, unsupported):
cursor movements 6: 6, 0, 0; screen features 15: 15, 0, 0; character sets 15: 12, 3, 0; double-sized characters 6: 6, 0, 0;
keyboard 6: 0, 0, 6; terminal reports 9: 8, 0, 0 (and a prompt); VT52 mode 4: 3, 0, 0 (and a prompt); insert and delete 14: 14, 0, 0;
known bugs 29: 29, 0, 0; reset 6: 5, 0, 1; non-vt100 345: 289, 2, 49 (and four prompts and one not exercised).

**Left out because a person is needed** (`suites/vttest/notes.txt` lists every entry that was not run and why): the keyboard menu's
tests that read typed keys (5.2 auto repeat, 5.3 layout, 5.4 cursor keys, 5.5 numeric keypad, 5.6 editing keypad, 5.7 function
keys, 5.8 answerback, 5.9 control keys), the keyboard layout test with a G0 selection (3.13), send/receive mode (11.1.2.1, local
echo of the keys pressed), the mouse tests (11.8.5.3 to 11.8.5.8) and the Tektronix 4014 tests (11.8.6, xterm's second window).
Where vttest waits for a few typed keys that need no person, the host types them through the engine's own key encoder (Return
for LNM, Backspace for DECBKM, a keypad key and a letter for the VT420 keyboard modes, `q` for DECUDK) and the title, the font
name and Bug D's answer as lines. The later models' menus list the earlier ones' as their first entry; each test runs once
(`notes.txt` says which entries were not walked again), the character-set choices (`Specify G0` to `G3`) are left to the
defaults, and the colour test and the origin/margin switches of the VT420 menus run with vttest's own start states.

### What vttest finds that the engine fails or does not support

Screen references are `<menu>/<screen>` (`3/003` is `suites/vttest/03-character-sets/003.txt`, `11/123` the non-VT100 menu's).

| gap | sequence | expected | the engine | screens |
|-----|----------|----------|-----------|---------|
| 8-bit characters and GR | `ESC % @` (leave UTF-8), then bytes 0xA0 to 0xFE; locking shifts into GR: `ESC ~` (LS1R), `ESC }` (LS2R), `ESC |` (LS3R) | the characters of the set designated to G1, G2, G3 in the right half | the input is UTF-8 only, by design: every such byte is U+FFFD; LS2 and LS3 into GL (`ESC n`, `ESC o`) and the single shifts work | 3/003, 3/008, 3/013, 11/187 |
| keyboard position mode | DECKPM (`CSI ? 81 h`): keys report their position | a VT510 key reports its position, not its character | not built: xterm has no such mode either and no program asks for it (`docs/input.md`). LNM (Return sends CR LF) works since the encoder got `InputModes::new_line` | 11/135 to 139 |
| printer | media copy: `CSI ? 9 i`, `CSI ? 8 i`, `CSI ? 5 i`, `CSI i`, `CSI ? 1 i`, `CSI ? 11 i`, DECPEX, DECPFF | text and pages go to the printer | there is no printer, by design: media copy is ignored, the printer controller mode (`CSI 5 i` to `CSI 4 i`) included, so its data is drawn. Swallowing it would let a stray `CSI 5 i` silence the terminal for good (a reset would be swallowed too); modern terminals ignore it as well. DSR says there is no printer | 11/012 to 021 |
| rectangle checksum | DECRQCRA: `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` | the VT420's: the two's complement of the sum, with the video attributes | xterm's XTCHECKSUM variant on purpose (the plain sum of the code points, no attributes), which esctest2 reads the screen with: every character's checksum mismatches, as does the whole page's (`4273`, expected `EAED`) | 11/186, 11/187 |
| status line | DECSSDT (`CSI Ps $ ~`), DECSASD (`CSI Ps $ }`) | text written while the status line is selected goes there | the engine draws no status line, by design: what is written to it is dropped, and the main screen and cursor are left alone (`screen/status_line.rs`) | 11/073 to 081 |
| settings the engine does not have, asked by DECRQSS | DECPRO, DECTTC, DECTLTC; the VT520's DECATC, DECAC, DECCRTST, DECDLDA, DECSEST, DECARR, DECSTGLT, DECSZS, DECSPMA, DECTME, DECUS; xterm's modifyKeyboard, modifyCursorKeys, modifyFunctionKeys and the UTF-8 title modes | a valid reply | `DCS 0 $ r ST`, an invalid request, which is what a terminal without the setting answers (vttest prints `failed` for an answer without the request's final byte). modifyOtherKeys and the hexadecimal title modes are answered | 11/058, 11/067, 11/068, 11/210 to 220, 11/311 to 313, 11/317, 11/318 |
| font | OSC 50 (set and query) | a window feature | ignored, no reply | 11/321 to 323 |
| user-defined keys, soft characters | DECUDK, DECDLD | keys that echo their labels, a soft font | not implemented (and DECDLD was not exercised: it needs a font file) | 11/022, 11/026 |
| guarded areas of ISO 6429 | SPA, EPA with ERM | "there may be" a box | none | 11/285, 11/300 |
| keyboard LEDs and the confidence test | DECLL, DECTST | hardware | ignored, as xterm | 5/001 to 006, 10/004 |

Found by vttest and fixed (the engine's `.vtt` cases cite the screens): double-width and double-height lines (menus 4, 8, 9), REP after a
control sequence, text for the status line, DECREQTPARM, DECSCPP, Return in new line mode, XTTITLEPOS and
DECRQSS for modifyOtherKeys and the title modes, DECRQSS at the VT100 level, DEL in a 96-character set. Judged right after all: DECCARA and
DECRARA streams run to the screen's edges, not the margins (the VT510 manual: "not affected by page margins"; xterm's the same), though
vttest's text says "wrapping at the margins" (11/156, 11/165).

What passes beyond the obvious: 80 and 132 columns (DECCOLM) and the window operations that resize; wrap, tabs, origin mode and
scrolling regions; the VT100 bugs (all of them absent); character sets B, A, 0, 1 and 2, ISO Latin-1 in GL through SI, SO, LS2, LS3
and the single shifts, and the national sets; insert and delete of characters, lines and (DECIC, DECDC) columns; the reports (DSR,
DA 1 to 3, DECRPM, DECRQSS for the modes it has, DECCIR, DECTABSR, DECRSPS, DECTSR, window reports); left and right margins with
every editing and movement test; the rectangular operations (DECCARA, DECCRA, DECERA, DECFRA, DECRARA, DECSERA), with and without
margins; protected areas (DECSCA with DECSED, DECSEL); ISO 6429 colours, background colour erase, SGR 22 to 27; cursor styles
(DECSCUSR); the alternate screens (47, 1047, 1049); XTVERSION.

## The engines

Each is a black box behind its public API: nothing of their source is read here.

| engine | what | through | pinned |
|--------|------|---------|--------|
| `alacritty` | `alacritty_terminal`, what Wormhole ran before its own engine | its documented Rust API (`src/engines/alacritty.rs`) | `Cargo.lock` |
| `ghostty` | libghostty-vt, Ghostty's terminal as a C library | its C headers (`include/ghostty/vt/`), through `shim/ghostty_shim.c` and `build.rs` | `ghostty.pin` |
| `wezterm` | `wezterm-term`, the model WezTerm draws from | its documented Rust API (a git dependency) | `Cargo.toml` (a commit of main; the last release is from February 2024) |
| `vt100` | the `vt100` crate, a parser that keeps a screen (tmux and screen like tools use it) | its documented Rust API | `Cargo.lock` |

**Ghostty.** `./fetch-ghostty.sh` clones Ghostty at the commit in `ghostty.pin` into a cache
(`~/Library/Caches/wormhole/vt-compare/`, or `$WORMHOLE_VT_COMPARE_CACHE`), fetches the Zig it asks for
if the installed one is not exactly that version (checksum-verified; 0.16.0 today), and builds
`libghostty-vt.a` (ReleaseFast, a minute). `build.rs` links it. Without it the engine is left out and
everything else works (`vt-matrix --list` says so). The commit is not a release: v1.3.1, the latest,
has no terminal in its C API (only the OSC, SGR, key and paste helpers); `ghostty_terminal_new` and
the grid references came after it.
`GHOSTTY_VT_DIR=<folder with lib/ and include/>` points the build somewhere else.

**What the host answers.** An engine is a parser and a screen; the window around it answers some
queries. The adapters answer the way the alacritty one does: the cell size is 8x16 pixels, the
colors are `DEFAULT_COLORS` and xterm's palette. Ghostty also gets device attributes (a VT220 with
ANSI color and the clipboard); the conformance cases accept any.

**Defaults that matter.** `wezterm-term` runs with its configuration defaults (so Unicode 9 widths,
which is what the app has unless told otherwise, and no kitty keyboard), `vt100` and libghostty-vt
as they come.

## Reading a matrix

A cell is `passes/cases asked`. A case an adapter cannot ask is **n/a** for that engine, not a
failure, and counts in neither number:

| engine | n/a when the case needs |
|--------|-------------------------|
| `vt100` | replies (it has no output channel), hyperlinks, underline colors, any attribute other than bold, dim, italic, underline and inverse |
| the others | nothing: they answer everything |

Things that look like failures but are how the engine is made:

- **ghostty** keeps history in pages and prunes a page at a time, so a history limit is an estimate
  (`scroll: the history keeps at most the lines it is allowed`).
- **wezterm** answers a query from a thread of its own, some time after the bytes are fed. The
  adapter waits for it with a key nobody types (a key goes through no parser), so a reply is never
  missed, but the order of replies relative to the key encoder is the model's.
- A cursor waiting to wrap is one past the last column in wezterm and vt100; the adapters report the
  last column, as a window draws it.
- **wezterm**'s borrowing line reader panics on a screen that has scrolled down (IL, RI, SD) at the
  pinned commit; the adapter reads copies of the lines instead.

`Where the engines disagree` is the interesting part: a case from published text that engines
fail is a gap in that engine; a `convention` that most engines fail is a choice of ours to look at
again. `C` marks conventions, `S` published text; `✗ (known)` is a failure the case file lists as
`known-fail:`. Failures that appear only when the input is cut into single bytes (a chunk boundary
between a base character and its combining mark) say so in `--json` (`only_when_split`).

## vt-bench

`vtbench`'s method: a fresh engine per pass, the stream fed in 64 KiB pieces, the fastest of 5
passes; allocator calls per MB over the first pass and over a second pass on the same engine
(steady state: zero is the goal). Use `--release`. For the Rust engines the allocator is the global
one; for Ghostty it is the allocator libghostty-vt is given (it also maps page memory directly,
which no allocator sees, so its figure is calls, not memory). Numbers from one machine and one run
are a guide, not a verdict. The run kept in `results/` (the matrix as markdown, the throughput tables
and their JSON) was made on a machine with other work going on, a load average of 8 against 18
cores; the end-to-end results are in `tools/bench/results/`.

## Adding an engine

An adapter in `src/engines/<name>.rs` implementing `wormhole_vt_bench::Engine` (its documentation
has the rules), a line in `src/engines/mod.rs` (`engine_names`, `for_each_engine`,
`visit_engine`, `engine_info`) and, if it cannot answer something, an entry in `src/caps.rs`.

## Running esctest2 without a window

`cargo run --release -p wormhole-vt-bench --bin esctest-host -- --out /tmp/wormhole.jsonl` runs esctest2
against Wormhole's engine with no app and no window: the suite runs on a pseudo-terminal that
`suites/pty_relay.py` keeps, and the host process is the terminal (the engine, the clipboard, a fake
window for the window operations and a black-on-white theme, as esctest2 assumes). It writes the same
JSON lines as `suites/esctest_run.py` (`--include <regex>` runs a part; about 90 seconds for the whole
suite) and `suites/esctest_compare.py` compares two result files. The in-app run differs only in the
theme's colors, which the host plays as xterm's own, and in windows being real.

## The suites as a gate

`scripts/suites-gate.sh` is the headless esctest2 run and the headless ucs-detect run together (and then vttest, see above), compared
test by test with `expected/esctest.tsv` and `expected/ucs.tsv` by `gate.py`: every test that passes in
the expectation must still pass, and the ones that regressed are named. A known failure that starts to
pass is reported and the expectation is left for a person to update (`scripts/suites-gate.sh --update`,
review the diff, commit it; do the same after moving a pin in `suites.pin`). `scripts/check.sh` runs it
last when `crates/wormhole-vt`, `crates/wormhole-vt-bench` or core's `terminal/` changed, and with
`--all`. It takes about a minute (esctest2 is most of it; ucs-detect needs two seconds; both run at
once). Without the cache and without a network (cloud sessions) it skips itself with a warning.
Today: esctest2 544 of 567 (the 23 known failures are the Xcms colour forms of OSC 4, 5 and 10 to 19
and DA's printer port), ucs-detect 15,012 of 15,012 in 93 rows.
