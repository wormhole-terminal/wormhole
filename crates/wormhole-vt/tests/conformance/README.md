# Conformance suite

Plain-text cases for a terminal engine: the bytes a program sends and what the terminal must show,
where its cursor is, what its cells look like and what it answers. The files here (`*.vtt`) are
read by the runner in `crates/wormhole-vt-bench` (`src/vtt/`) and run against every engine adapter
(`src/engines/`): `wormhole-vt`; `tools/vt-compare` runs them against other terminals' engines
(alacritty, Ghostty, WezTerm, vt100), whose failures the `known-fail:` lines record.

```sh
cargo test -p wormhole-vt-bench --test conformance                     # all engines
VTT_FILTER="wide char" cargo test -p wormhole-vt-bench --test conformance -- --nocapture
VTT_VERBOSE=1 cargo test -p wormhole-vt-bench --test conformance -- --nocapture   # lists known failures
```

The runner lives with the bench crate, not in `wormhole-vt`, so that `wormhole-vt` never depends on
its tools (the engine adapter will depend on `wormhole-vt`, and a crate cannot be a dependency of
the crate that depends on it, not even for tests, without a cycle).

## Where the expected values come from

From published texts only, never from what another terminal does and never from another terminal's
tests or source:

- xterm's *Control Sequences* (`ctlseqs`), the reference for what xterm and everyone after it sends
  and answers;
- the DEC manuals on vt100.net: the VT100 User Guide and Technical Manual, the VT510 Programmer
  Reference (the per-function pages, such as CUD, DECSTBM, DECSC, DECSTR);
- ECMA-48, ISO 6429 (the standard behind the CSI functions);
- Paul Williams' DEC-compatible parser (`vt100.net/emu/dec_ansi_parser`);
- kitty's underline documentation (SGR 4:x, 58, 59), the OSC 8 hyperlink specification, the
  synchronized output (mode 2026) and terminal-unicode-core (mode 2027) specifications,
  Unicode (UAX #11 widths, UAX #29 clusters, §3.9 replacement characters), terminfo(5).

Every case has a `spec` line (the text that says it) or a `convention` line (see below).
`grep -c '^convention'` counts them. Two published texts disagree in places and the cases say which
one they follow: the VT510 manual's DECRPSS table has the validity digit the wrong way round (xterm
and the VT420 behaviour: `1` is valid), and DECXCPR is `CSI ? r ; c R` in xterm and
`CSI ? r ; c ; p R` in the VT510 manual (the case accepts both).

### Conventions

Some behaviour is universal but not spelled out in any of those texts: the *delayed wrap* (the
cursor waits in the last column and a cursor movement clears the wait; terminfo's `xenl` for
xterm-256color promises it, the DEC manuals only say that the 81st character wraps), erasing with
the background color (`bce`), clamping a position at the screen edge, scrolling a region in the
middle of the screen not feeding the history, rows that were wrapped and can be joined again, what
a resize keeps. Those cases use `convention <what is assumed>` instead of `spec`. They are the
choices to review: a convention is a decision about the engine, not something a manual says.

## Format

One file per area. Lines starting with `#` and blank lines outside a block are ignored. A case
starts with `test <name>` (names are unique across all files) and goes on until the next one.
Trailing blanks of a line are dropped; write `\s` for a blank that matters.

```
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

A case is a script, run from the top. `feed` and `resize` change the terminal; every other line
checks how it looks at that point.

### Header

| line | meaning |
|------|---------|
| `test <name>` | starts a case |
| `spec <text>` or `convention <text>` | where the expected values come from; exactly one, required |
| `size <cols> <rows>` | the screen's size; required |
| `scrollback-limit <n>` | lines of history the engine keeps (default 100) |
| `screen-checksum allow` | the engine is told to answer DECRQCRA, which it keeps off otherwise (a terminal that does not have the switch ignores it) |
| `window-ops allow` | the engine is told to act on the window operations (XTWINOPS 1 to 10, DECSLPP, DECSNLS), which it keeps off otherwise |
| `title-report allow` | the engine is told to answer the title and icon name reports (XTWINOPS 20 and 21), which it keeps off otherwise |
| `known-fail: <engine> – <reason>` | the named engine gets this case wrong, and how (the dash may be `–`, `—`, ` - ` or ` -- `); repeat for several engines |

### Input

| line | meaning |
|------|---------|
| `feed <text>` | bytes for the terminal, with the escapes below |
| `repeat <n> <text>` | the same, `n` times (long payloads) |
| `resize <cols> <rows>` | the window is resized |

Escapes in `feed`, `repeat` and in expected text: `\e` (ESC), `\a`, `\b`, `\t`, `\n`, `\v`, `\f`,
`\r`, `\0`, `\s` (a blank), `\\`, `\xNN` (a raw byte in `feed`, so invalid UTF-8 can be written; the
character U+00NN, ASCII only, in expected text) and `\u{…}` (a character, UTF-8 encoded in `feed`).

Every case is run twice: with each `feed` handed over whole, and with it handed over one byte at a
time, so every escape sequence and UTF-8 character is cut at every place. A correct engine shows no
difference. Splitting a sequence over two `feed` lines does the same on purpose.

### Checks

All positions count from 1, as the escape sequences do.

| line | meaning |
|------|---------|
| `screen` + rows | the whole visible screen, row by row from the top; rows not listed must be blank |
| `row <n> \|text\|` | one row |
| `cursor <row>,<col> [visible\|hidden]` | where the cursor is (`*` for the row: any), and whether it is shown |
| `cell <row>,<col> key=value …` | properties of one cell, below |
| `reply <text>` | the bytes the terminal sent since the last `reply`, exactly |
| `reply~ <text>` | the same, where `*` matches any run of bytes (`\x2a` for a real star) |
| `reply none` | it sent nothing |
| `title <text>` / `title none` | the window title set by OSC 0 or 2 |
| `alt-screen on\|off` | whether the alternate screen shows |
| `scrollback` + rows | the history, oldest line first; exactly these lines |
| `scrollback-len <n>` | how many lines the history holds |
| `lines` + rows | the text as logical lines: history and screen together, rows that continue by wrapping joined; what a resize must not change |
| `wrapped <row> yes\|no` | whether the row continues in the next one because the text wrapped |

Rows in a block are written between `|` and `|`; the closing bar shows trailing blanks, which are
not significant (`|ab  |` and `|ab|` are the same). A wide character is written once and its second
cell is left out; combining marks go right after their base (write them as `\u{301}` so they can be
seen).

Cell properties: `text=` (the cell's text, a blank is `\s`, a cluster is all of it, the second half
of a wide character is empty), `width=` (1, 2 for the first half of a wide character, 0 for the
second), `fg=`, `bg=`, `ul=` (`default`, `idx:N` for palette slot N: SGR 31 is `idx:1`, 91 is
`idx:9`, `38;5;N` is `idx:N`; or `rgb:RRGGBB`), `attrs=` (the exact set, comma separated, or `none`:
`bold dim italic underline double-underline curly-underline dotted-underline dashed-underline blink
inverse hidden strike overline`) and `link=` (the OSC 8 target or `none`). The colors are what the
program asked for: `inverse` is an attribute, not a swap of `fg` and `bg`.

### Results

A case that fails prints its place, what it is based on, the input, the step and a diff:

```
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

A `known-fail` for the engine that runs counts as passing when the case fails. When it passes the
run reports `known-fail marks that can go` and still succeeds: the mark is stale, not the engine
broken. The `known-fail:` lines for an engine are that engine's list of mistakes; the new engine is
expected to pass every case without one.

## Adding cases

Take the value from a published text, quote where in `spec`, and write the case so that it checks
one thing. Prefer a `reply~` pattern over an exact reply when the text leaves a choice (which
terminal a DA reply names). Do not copy a case, its expected screen or a reason from another
terminal's test suite, and do not take an expected value from running a terminal: run the case
only to find out which engines fail it. If an engine disagrees, check the text again before
writing `known-fail:`.

Cases in a file are grouped by area: `cursor`, `wrap`, `erase`, `edit`, `scroll`, `margins`,
`origin`, `tabs`, `save-restore`, `charsets`, `altscreen`, `sgr`, `unicode`, `reports`, `osc`,
`reset`, `modes`, `parser`, `resize`.
