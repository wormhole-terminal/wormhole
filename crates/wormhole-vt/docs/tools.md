<!--
Commands and paths are those of the repository today (crates/wormhole-vt, crates/wormhole-vt-bench,
tools/vt-compare, scripts/*.sh); re-check them if files move.
-->

# Tools

What is in the repository for looking at the engine, timing it, and testing it, and the command for each.
The first two groups are for when something is wrong or slow; the rest are the checks that keep it right.
Commands run from the repository's root unless they say otherwise. The ones that time anything need
`--release` (a debug build says nothing about speed).

## Looking at a stream

**`vtdump`** (an example of the crate) lists what is in a byte stream, one item per line: the offset it
starts at, its bytes (`\e` is ESC), and its name from xterm's *Control Sequences* (`SGR 1;31 (bold, fg
red)`, `DECSET 2004 (bracketed paste)`, `OSC 8 hyperlink "https://…"`, `DCS q (Sixel, 1234 bytes)`).
It is the first thing to reach for when a program draws wrong: it shows what the program really sent.
With `--engine` it also feeds each item to a terminal and prints what came back, the replies
(`→ reply \e[2;1R`) and the events (`→ event Title`). `--no-text` leaves out printed text and
`--offset A..B` keeps the items that touch those bytes.

```sh
cargo run -q -p wormhole-vt --example vtdump -- session.raw
cat session.raw | cargo run -q -p wormhole-vt --example vtdump -- --no-text --offset 0..4096
cargo run -q -p wormhole-vt --example vtdump -- --engine --size 120x40 session.raw
```

**`vtreplay`** feeds a recording to a terminal of a given size and prints the screen as text, with a
`^` line under the cursor's row. Print the end state, the state after a byte offset (`--at`), a screen
every N bytes (`--every`), or ask where a text first shows (`--grep`). Pair it with `vtdump --offset` to
see which sequences led to a wrong screen. A recording made by `scripts/vt-record.sh` has a `.meta`
file beside it, and its size is used when `--size` is not given.

```sh
cargo run -q -p wormhole-vt --example vtreplay -- --size 120x40 session.raw
cargo run -q -p wormhole-vt --example vtreplay -- --at 4096 session.raw
cargo run -q -p wormhole-vt --example vtreplay -- --grep "error[E0308]" session.raw
```

`scripts/vt-record.sh <name> -- <command>` records a real program's output (vim, htop, a coding agent) to
`tools/vt-corpus/`, for these two tools and for the benchmarks.

## Measuring speed

**`vtbench`** times the engine on whole corpora (printable text, colour churn, full-screen redraws,
scrolling regions, hyperlinks, picture protocols, a coding agent's session, and any recording): MB/s
and allocator calls per MB, the fastest of five passes, the stream fed in 64 KiB pieces. It is the
number to quote, and `--json` makes a baseline to compare a change against.

```sh
cargo run --release -p wormhole-vt-bench --bin vtbench
cargo run --release -p wormhole-vt-bench --bin vtbench -- --corpus log-lines --runs 9
cargo run --release -p wormhole-vt-bench --bin vtbench -- --json > before.json
```

**The `divan` benchmarks** (`crates/wormhole-vt/benches/engine.rs`) time single paths, to find which
function got slower: the parser alone (ASCII, UTF-8, SGR, OSC), the terminal's `feed` (plain text, a
scrolling region, wide characters and emoji under mode 2027, hyperlinks, OSC payloads), reflow of a full
history, and copying the viewport row by row. Under each time divan shows the allocations of one run, so
a hot path that starts to allocate shows up as a number where there was `0`. The whole set takes under
a minute.

```sh
cargo bench -p wormhole-vt
cargo bench -p wormhole-vt -- feed            # the ones with "feed" in the name
```

**`tools/vt-compare`** runs the same corpora (`vt-bench`) and the same conformance cases (`vt-matrix`) on
other engines, `alacritty_terminal`, `wezterm-term`, `vt100` and libghostty-vt, as black boxes. It is
its own Cargo workspace, so those dependencies stay out of the engine's.

```sh
cd tools/vt-compare
cargo run --release --bin vt-bench
cargo run --release --bin vt-matrix
```

## Profiling

**`scripts/profile-vt.sh`** builds the `profile_feed` example with the `profiling` profile (the release
build with line tables, so a profiler names functions and lines) and runs it under
[samply](https://github.com/mstange/samply) (`cargo install --locked samply`; on macOS run
`samply setup` once), which opens the Firefox Profiler in the browser, or under Instruments' Time
Profiler with `--tool xctrace`. `profile_feed` loops a generated stream through a terminal for a few
seconds, with the reader's `copy_viewport` after every chunk as a host would do. Read the profile with
the call tree inverted; allocator calls under `feed` are a finding.

```sh
scripts/profile-vt.sh --corpus sgr --seconds 10
scripts/profile-vt.sh --tool xctrace --corpus links --no-open
scripts/profile-vt.sh --list                    # the streams
```

`--alloc` runs no profiler and prints the allocator calls per phase instead: the parser alone, the first
and the second pass of a terminal (the second is the steady state), and the reader's copy, with the
bytes still held after each. It is the cheapest way to see where allocations come from.

```sh
scripts/profile-vt.sh --alloc --corpus links
```

## Checking correctness

**The test suite** (`cargo test -p wormhole-vt`) holds the unit tests, the allocation counters
(`tests/*_allocations.rs`: the parser, the grid and the repaint writer allocate nothing in the steady
state, a count and not a time, so they hold on any machine), the structure test (layers depend only
downwards, files stay under 1,000 lines) and the invariants below.

**The invariant tests** (`tests/invariants/`, `tests/invariants_memory.rs`) check properties that must
hold for every input, on streams generated from a seed: a stream fed whole, cut at every byte and cut
in random pieces leaves the same state, replies and events; resizing through any sizes and back
loses no character; every reply is a well-formed sequence that carries no text the request chose;
and hundreds of megabytes of hostile input (strings that never end, endless parameters, picture
garbage, floods nobody collects) leave the terminal within the caps in [Security](security.md). A
failure names the seed. The heavy variants are ignored by default:

```sh
cargo test -p wormhole-vt --test invariants -- --ignored
cargo test -p wormhole-vt --release --test invariants_memory -- --ignored
```

**The conformance runner** runs the `.vtt` cases in `crates/wormhole-vt/tests/conformance/` on the engine,
each whole and byte by byte; every case cites the text it follows. See [Conformance](conformance.md) for
the format and how to add one.

```sh
cargo test -p wormhole-vt-bench --test conformance
VTT_FILTER="wide char" cargo test -p wormhole-vt-bench --test conformance -- --nocapture
```

**The outside suites gate** (`scripts/suites-gate.sh`) runs esctest2 and ucs-detect against the engine
with no window and compares them, test by test, with the kept expectation: a test that passed must still
pass. About a minute; needs the suites fetched once (`tools/vt-compare/fetch-suites.sh`).

**Differential and bisimulation tests** (`cargo test -p wormhole-vt-bench`) compare the engine with
`alacritty_terminal` on random streams, and rebuild a terminal from its repaint bytes to check that it
stays equal to the original.

**Fuzzing** (`scripts/fuzz.sh`) runs the four libFuzzer targets (`parser`, `split`, `term`, `graphics`)
with overflow checks and the terminal's invariants checked after every step. It needs a nightly
toolchain and `cargo install cargo-fuzz`, and is run by hand. A finding becomes a regular test.

```sh
scripts/fuzz.sh seeds                      # once: the starting corpus
scripts/fuzz.sh run term -m 20             # one target, 20 minutes
scripts/fuzz.sh repro term crates/wormhole-vt/fuzz/artifacts/term/crash-…
```

**Coverage** (`scripts/coverage.sh`) prints line and branch coverage per module from the engine's tests,
the conformance runner and the bench crate's tests (`--suites` adds the outside suites, `--missing` lists
the lines not run). It needs `cargo install cargo-llvm-cov` and a nightly toolchain with
`llvm-tools-preview`.

```sh
scripts/coverage.sh --files 25
```
