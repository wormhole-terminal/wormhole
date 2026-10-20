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

`./fetch-suites.sh` fetches both at the commits in `suites.pin` into the cache (not into this repository)
and makes a Python venv for them; `scripts/suite-compare.sh` calls it. `suites/` holds our side:
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

`scripts/suites-gate.sh` is the headless esctest2 run and the headless ucs-detect run together, compared
test by test with `expected/esctest.tsv` and `expected/ucs.tsv` by `gate.py`: every test that passes in
the expectation must still pass, and the ones that regressed are named. A known failure that starts to
pass is reported and the expectation is left for a person to update (`scripts/suites-gate.sh --update`,
review the diff, commit it; do the same after moving a pin in `suites.pin`). `scripts/check.sh` runs it
last when `crates/wormhole-vt`, `crates/wormhole-vt-bench` or core's `terminal/` changed, and with
`--all`. It takes about a minute (esctest2 is most of it; ucs-detect needs two seconds; both run at
once). Without the cache and without a network (cloud sessions) it skips itself with a warning.
Today: esctest2 544 of 567 (the 23 known failures are the Xcms colour forms of OSC 4, 5 and 10 to 19
and DA's printer port), ucs-detect 15,012 of 15,012 in 93 rows.
