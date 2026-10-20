<!--
API names assumed here: Terminal::{new, feed, take_replies, events, modes, resize, pin, pin_point, unpin,
span...}, Modes, Event, Pin, Span (D-16, public anchor API; today PinId and PinnedSpan), Line, Column, Point,
Cell, Flags, Color, Policy.
-->

# Concepts

This chapter explains the model: what a terminal is to this library, what it keeps, and how it talks to
the host. Read it once before the other chapters.

## The terminal is a state machine

A `Terminal` is the state of one terminal screen. You give it bytes; it changes its state. It never
reads, writes or waits on its own.

```text
program output --feed--> Terminal --> screen, history, cursor, modes, title ...
                            |
                            +--> replies  (bytes for the program's input)
                            +--> events   (things the host should do or know)
```

Nothing happens between calls. There are no timers, no threads and no callbacks. If a program prints
`ESC [ 3 1` and stops, the terminal waits for the next call to `feed`. A sequence may be cut at any byte,
and a UTF-8 character may be cut in the middle. Feeding a stream whole or one byte at a time gives the
same screen. The conformance suite checks that for every case.

## The screen and the history

The terminal has two **screens**: the main screen and the alternate screen (modes 47, 1047 and 1049).
Full-screen programs such as editors switch to the alternate screen and back. Each screen has its own
cells and its own pictures. The cursor, the pen, the character sets, the margins and the tab stops are
shared by both. The saved cursor (DECSC) and the kitty keyboard flags are kept per screen.

The main screen has **history**: rows that scrolled off the top. The alternate screen has none.

Lines are addressed by `Line`. Line 0 is the top row of the live screen. Lines above it are negative:
`Line(-1)` is the newest row of history. A `Point` is a `Line` and a `Column`. Columns count from 0.

The history is capped in **bytes**, not lines, because rows differ in width. A cap of N lines becomes
about N rows of the current width plus a small slack. Rows are dropped a page at a time, oldest first, so
the history may hold up to a few hundred lines fewer than the cap allows, never more. See `embedding.md`
for sizing.

Scrolling the whole screen up appends to the history; no cell moves. Scrolling only a part of the screen
(a region that starts below the top row, or insert/delete line) does not feed the history.

## Cells and styles

A cell holds one character and how it looks. When you read a cell you get:

- `c`, the character. A blank cell is a space.
- `fg` and `bg`, each a `Color`: `Named` (the default foreground and background, and the 16 palette
  names), `Indexed(u8)` (the 256-colour palette) or `Rgb`.
- `flags`: bold, dim, italic, one of five underline styles, blink, inverse, hidden, strikeout, overline,
  and the layout flags below.

Colours are what the program asked for. `inverse` is a flag, not a swap of `fg` and `bg`; the renderer
decides what it looks like.

Things that are rare on a cell are not in the cell. They live beside the row, in `RowExtras`: the
zero-width characters that combine with the cell's character (accents, variation selectors, the joiners
of an emoji sequence), an underline colour of its own, and a hyperlink. A row with none of them costs
nothing to read.

Internally, cells are eight bytes and styles are interned per page, so a screen of text with a few
styles is small. You do not see this, and it can change in any release.

### Wide characters

A wide character (an ideograph, most emoji) takes two cells. The first cell holds the character and has
the `WIDE_CHAR` flag. The second has `WIDE_CHAR_SPACER` and holds nothing. If a wide character does not
fit at the end of a row, the engine leaves a blank with `LEADING_WIDE_CHAR_SPACER` there and puts the
character on the next row.

With grapheme clustering on (mode 2027, on by default), a cluster such as a flag, a family emoji or a
letter with several marks is one group of cells, as wide as its first character, or 2 when a variation
selector or a spacing mark makes it wide. It is never wider than 2. With mode 2027 off, every code point
is on its own and marks attach to the cell before them. Programs that count code points can turn it off.

Widths come from tables generated for Unicode 18 (`unicode::UNICODE_VERSION`). The tables are also
public (`unicode::width`, `unicode::breaks`).

## The delayed wrap

When a character fills the last column, the cursor does not move to the next row. It stays on the last
column with a **wrap pending** flag. The next printable character wraps first, then prints. Anything that
moves the cursor clears the flag (terminfo calls this `xenl`), and so does an edit of the line under it.

This is why a program can print exactly 80 characters on an 80-column screen followed by a newline
without getting a blank line. You will meet it when you draw the cursor: with a wrap pending, the cursor
is reported on the last column.

A row that was continued by wrapping is marked as **soft-wrapped**. Resizing rewraps soft-wrapped lines.
Hard line breaks stay.

## Modes

Programs switch behaviour with modes (`CSI ? Ps h` and `l`, and the ANSI forms `CSI Ps h` and `l`).
`Terminal::modes()` returns them as one set, `Modes`. The ones a host acts on:

| Modes | What the host does with it |
|---|---|
| `ALT_SCREEN` | Whether the alternate screen shows. Full-screen programs; a host often turns the scroll wheel into arrow keys here (`ALTERNATE_SCROLL`) |
| `SHOW_CURSOR`, cursor shape | Draw or hide the cursor |
| `APP_CURSOR`, `APP_KEYPAD` | Choose key encodings (`input.md`) |
| `MOUSE_*` | Whether and how to report the mouse |
| `BRACKETED_PASTE` | Wrap pasted text in markers |
| `FOCUS_REPORT` | Tell the program when the window gains or loses focus |
| `KEYS_*` | The kitty keyboard flags in force |
| `REVERSE_VIDEO` | Draw the screen with foreground and background swapped (DECSCNM) |
| `SYNC_OUTPUT` | A program is in the middle of an update; see `embedding.md` |
| `GRAPHEME_CLUSTERS` | Whether widths follow clusters |

Many more modes are *recorded*: the terminal remembers them and answers DECRQM, but they change nothing
here (for example the VT510's hardware and set-up modes). Modes belong to the terminal, not to a screen:
insert mode set on the main screen is still set on the alternate one.

A reset (RIS, `ESC c`) returns the modes to their starting values and emits `Event::FullReset`. Mode 40,
which only permits switching between 80 and 132 columns, stays as the program set it. A soft reset
(DECSTR, `CSI ! p`) resets the keyboard modes and the ones that decide how text is placed (insert,
origin, margins, reverse wrap). Mouse, paste and focus modes belong to the program that set them and
stay. The starting values follow xterm, with two choices of this engine: grapheme clusters (2027) and
the wheel-sends-arrows mode (1007) start on.

## The reply channel

Many sequences are questions: where is the cursor (`CSI 6 n`), what are you (`CSI c`), is mode 2004 set
(`CSI ? 2004 $ p`), what colour is the background (`OSC 11 ; ?`). The answer is bytes the program reads
from its input. The terminal queues them as **replies**, in the order they were asked, and the host
writes them to the PTY (`Terminal::take_replies`).

Two rules matter:

- **Order.** Replies come out in the order of the questions. When a reply must wait for the host
  (a picture the host has to decode, a clipboard read), the replies behind it wait too, so the program
  always reads its answers in the order it asked.
- **Bounded.** A host that never collects replies does not make the terminal grow: past 1 MiB new replies
  are dropped, whole. A reply is never cut in the middle.

The terminal builds replies from its own state. See `security.md`.

## Events

Some sequences ask the host to do something that is not drawing. They queue **events**, taken in order
with `Terminal::events()`:

- `Title` (read the new title with `title()`), `IconName`, `Cwd` (the shell's folder, OSC 7), `Bell`
- `Notification` and `KittyNotification` (OSC 9, 777, 99), `Progress` (OSC 9;4)
- `ClipboardStore` and `ClipboardLoad` (OSC 52). A load asks the host to answer with
  `Terminal::clipboard_reply`.
- `Mark` (OSC 133 shell marks: prompt start, command start, output start, finished with an exit code)
- `ImageData` and `ImageFree` (pictures to decode and release)
- `ColumnsRequested` (DECCOLM) and `WindowOp` (window control, only when `Policy` allows)
- `FullReset`

`Event` is `#[non_exhaustive]`: match with a wildcard arm.

Events carry what the program sent, raw. The text of a notification or a clipboard store is the
program's. The host must treat it as untrusted (`security.md`).

Like replies, events are bounded: past 16,384 queued events new ones are dropped, and a run of the same
event (a bell, a title, a folder said twice) counts once. A picture's pixels, a picture's release and a
reset are never dropped, so the host cannot end up holding pixels nobody can show.

## Pins and spans

A host often needs to remember a place in the text: a search hit, a selection, the start of a command, a
decoration. A row number is not enough, because rows move when the screen scrolls and when the width
changes.

A **pin** is a reference to a row that follows its text. It survives scrolling that appends to the
history, and it survives reflow when a resize rewraps lines. It dies when its row is dropped from the
history; asking for its position then returns `None`. A **span** is a start pin and an end pin.

One limit to know: a scroll of only part of the screen (a region, insert or delete line) moves text
between rows, and a pin stays on its row. Unpin what you no longer need.

## Policy

Three kinds of sequence are dangerous when a program you do not trust sends them: ones that move or
resize the window, ones that read the window's title back, and ones that read the screen's contents back
(DECRQCRA). `Policy` is the one place that switches them on. The default is locked down: all off. See
`security.md`.

## What the engine does not do

- It does not read from or write to a PTY, a file or a socket.
- It does not decode PNG or JPEG. It hands the encoded bytes to the host (`rendering.md`).
- It does not draw, measure fonts or know about pixels, except the cell size the host tells it, which it
  needs for a few reports and for placing pictures.
- It does not know about your application. There is no application-specific sequence in it; use an
  extension (`extensions.md`).
