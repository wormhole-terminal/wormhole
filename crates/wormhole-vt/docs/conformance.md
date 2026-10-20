<!--
No target-API names are used in this chapter. Commands and paths are those of the repository today
(tests/conformance, crates/wormhole-vt-bench, tools/vt-compare, scripts/*.sh); re-check them if the phases move files.
-->

# Conformance

How we know the engine does what the specifications say, and how you can check it yourself.

The tests described here live in the Wormhole repository, not in the published crate: the runner and the
comparison tools are separate crates that depend on the engine (a crate cannot be a dependency of the
crate that depends on it, not even for tests, without a cycle), and some of them pull in other terminals'
engines.

## The `.vtt` cases

A case is a few lines of plain text: the bytes a program sends, and what the terminal must show, where the
cursor must be, what cells look like and what it answers. There is one file per area in
`crates/wormhole-vt/tests/conformance/` (`cursor`, `wrap`, `erase`, `edit`, `scroll`, `margins`, `origin`,
`tabs`, `save-restore`, `charsets`, `altscreen`, `sgr`, `unicode`, `reports`, `osc`, `reset`, `modes`,
`parser`, `resize`, `rect`, `rect-attributes`, `checksum`, `window`, `graphics`, `conformance-levels`).

```text
test CUP moves to row;column
spec xterm ctlseqs: CSI Ps ; Ps H
size 10 4
feed \e[3;5Hx
cursor 3,6
screen
|          |
|          |
|    x     |
```

A case is a script, run from the top. `feed` and `resize` change the terminal; every other line checks
how it looks at that point. The checks are `screen`, `row`, `cursor`, `cell` (text, width, colours,
attributes, link), `reply`, `title`, `alt-screen`, `scrollback`, `lines` (the history and screen as logical
lines, which a resize must not change) and `wrapped`. The full format is in the README next to the files.

### Every case cites its source

Every case has exactly one `spec` line or one `convention` line.

- A **`spec`** line quotes the published text the expected values come from: xterm's *Control Sequences*
  (`ctlseqs`), the DEC manuals on vt100.net (the VT100 User Guide and Technical Manual, the VT510
  Programmer Reference), ECMA-48, Paul Williams' DEC-compatible parser, kitty's underline documentation,
  the OSC 8 hyperlink specification, the synchronized output (2026) and terminal-unicode-core (2027)
  specifications, Unicode (UAX #11 widths, UAX #29 clusters, §3.9 replacement characters) and
  terminfo(5).
- A **`convention`** line names behaviour that is universal but that no published text spells out: the
  delayed wrap, erasing with the background colour (`bce`), clamping a position at the screen edge, rows
  that were wrapped and can be joined again, what a resize keeps. Conventions are decisions about the
  engine. They are the cases to review when you disagree with a behaviour.

The expected values come from **published text only**. They never come from what another terminal does,
and never from another terminal's tests or source. When two published texts disagree, the case says
which one it follows. Two examples: the VT510 manual's DECRPSS table has the validity digit the wrong way
round (xterm and the VT420 say `1` is valid), and DECXCPR is `CSI ? r ; c R` in xterm and
`CSI ? r ; c ; p R` in the VT510 manual (the case accepts both).

### Every case runs twice

Once with each `feed` handed over whole, and once with every byte handed over alone. Every escape sequence
and every UTF-8 character is cut at every place. An engine that handles chunk boundaries gives the same
result both ways.

### Running them

```sh
cargo test -p wormhole-vt-bench --test conformance                    # all engines
VTT_FILTER="wide char" cargo test -p wormhole-vt-bench --test conformance -- --nocapture
VTT_VERBOSE=1 cargo test -p wormhole-vt-bench --test conformance -- --nocapture   # lists known failures
```

A failure prints its place, what it is based on, the input, the step and a diff:

```text
FAIL wrap.vtt:26 a line feed clears the pending wrap
  convention: a cursor movement control clears the delayed wrap flag, …
  input: "abcdefghij\nX"
  line 30 (screen):
    screen: expected (-) vs actual (+)
       1 |abcdefghij|
    -  2 |         X|
    +  2 ||
    -  3 ||
    +  3 |X|
```

To add a case, take the value from a published text, quote where in `spec`, and write the case so it
checks one thing. Prefer a pattern (`reply~`) over an exact reply where the text leaves a choice. Run it
on other engines only to learn which of them fail it, not to learn the expected value.

## The same cases on other engines

`tools/vt-compare` runs the cases against `alacritty_terminal`, `wezterm-term`, `vt100` and libghostty-vt,
each as a black box behind its public API. Its `vt-matrix` tool prints passes per area, the cases each
engine fails, and the cases where engines disagree. Where an engine gets a case wrong, the case records it
with a `known-fail:` line, so a regression in *that* engine's list is visible too.

```sh
cd tools/vt-compare
cargo run --release --bin vt-matrix              # add --markdown or --json
```

Ghostty needs `./fetch-ghostty.sh` first; without it that engine is left out and the rest works. A case an
engine's adapter cannot ask (the `vt100` crate has no reply channel and no hyperlinks, for example) is
**n/a** for it, and counts in neither number.

Read the matrix with this in mind: the cases are ours. An engine that fails one differs from our reading of
the cited text, which is worth a look, but it is not a verdict on the engine. The part to read closely is
*our conventions against the other engines*: a convention that most engines fail is a choice of ours to
reconsider. The numbers from the last kept run are in the README of the crate and, in full, in
`tools/vt-compare/results/`.

## Suites we did not write

Two outside suites are run in the real apps and against the engine alone:

| Suite | What it tests | Licence |
|---|---|---|
| [esctest2](https://github.com/ThomasDickey/esctest2) | About 560 automated tests of escape sequences against xterm's behaviour. It reads the screen back with DECRQCRA checksums | GPL-2.0 |
| [ucs-detect](https://github.com/jquast/ucs-detect) | How a terminal sizes Unicode: wide and narrow characters, ZWJ emoji, VS15 and VS16, flags, languages, by cursor position reports | MIT |

Neither is copied into the repository. A script fetches them at the commits pinned in
`tools/vt-compare/suites.pin`.

**esctest2** is judged against the behaviour it describes, not against xterm's known-bug annotations: a
terminal that does better than xterm is not failing. "Empty" and "blank" cells count as one. DECRQCRA is how
it reads the screen, so the harness switches `screen_checksum` on; a terminal that does not answer it
cannot be measured on the tests that check the screen, and those are left out of its count (the tables say
how many).

```sh
# the engine alone, no window; about 90 seconds for the whole suite
cargo run --release -p wormhole-vt-bench --bin esctest-host -- --out /tmp/wormhole.jsonl
# the engine against the kept expectations
scripts/suites-gate.sh
```

The gate compares a headless esctest2 run and a headless ucs-detect run, test by test, with the expected
results in `tools/vt-compare/expected/`. Every test that passes in the expectation must still pass. A
known failure that starts to pass is reported, and a person updates the expectation
(`scripts/suites-gate.sh --update`, review the diff, commit it). The gate takes about a minute and runs
as part of the repository's check script when the engine changes.

At the last kept run, esctest2 passes 544 of 567 headless. The 23 that do not are 21 Xcms colour forms
(`rgbi:`, `CIEXYZ:` and similar, whose expected numbers are one X server's colour profile) and two tests
that want a printer port in DA1. Both are decisions: colour names and device-independent colour forms
are not read, and Media Copy has nowhere to print to.

**ucs-detect** runs against the engine through a pseudo-terminal whose other end feeds the engine and writes
its cursor position reports back, with no window (`tools/vt-compare/ucs-headless.py`; a run takes a
second). `--clusters on|off` sets mode 2027 first, and `-- --all` tests every code point instead of the
contested ones.

To run both in installed apps (Wormhole, Ghostty, Alacritty, kitty, WezTerm, iTerm2, Terminal.app), one at
a time: `scripts/suite-compare.sh`, which writes a markdown table per suite.

## Other tests that guard the engine

- **Fuzzing** (`crates/wormhole-vt/fuzz`, by hand with `cargo fuzz`). Four targets: `parser` (any bytes,
  cut anywhere; the events must be the same whole, cut and byte by byte), `split` (two terminals, one fed
  whole and one cut: the same screen, history, cursor, replies and events), `term` (a script of bytes and
  host calls mixed: no panic, hang or runaway memory, and `check_invariants` after every step) and
  `graphics` (the picture protocols, with the host answering every picture, and the budgets must hold).
  A finding becomes a regular test next to the code it broke.
- **Property tests** run the same checks on generated streams on every `cargo test`.
- **Allocation tests** count allocator calls: the parser, the terminal, the grid, the selection and the
  repaint writer allocate nothing in the steady state. They count work, not time, so they hold on any
  machine.
- **Differential tests** compare the engine with `alacritty_terminal` on random streams, whole and cut,
  after removing what that engine gets wrong, and compare selecting and copying text.
- **Repaint bisimulation.** A terminal rebuilt from its repaint bytes and fed the rest of a stream must stay
  equal to the original at many cut points, for every corpus. The state comparison fails for any field the
  writer forgets, and a test checks the list of fields against the source so a new field cannot be missed.
- **Structure tests** check that the engine's layers depend only downwards and that files stay under 1,000
  lines.

## Speed

`vtbench` times the engine alone on a set of corpora (MB/s and allocator calls per MB; a fresh engine per
pass, the stream fed in 64 KiB pieces, the fastest of 5 passes), and `tools/vt-compare`'s `vt-bench` does the
same on the other engines with the same method. Use `--release`. Results from one machine and one run are a
guide, not a verdict.

```sh
cargo run --release -p wormhole-vt-bench --bin vtbench           # --help, --json
cd tools/vt-compare && cargo run --release --bin vt-bench        # --markdown, --json, --engine, --corpus
```
