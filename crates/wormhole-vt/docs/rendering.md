<!--
API names assumed here: Terminal::take_damage(&mut Damage) with Damage::{is_full, rows, contains, is_empty,
cursor_changed, colors_changed},
Terminal::copy_row(Line, &mut Vec<Cell>, Option<&mut RowExtras>) -> Option<RowMark> (exists today),
Terminal::copy_viewport(&mut Vec<ViewRow>, force) (exists today; may be folded into the Damage API),
Terminal::cursor() -> Cursor/TerminalCursor, Terminal::{start_selection, update_selection, selection_range,
selection_text, drop_empty_selection, take_selection, link_span, has_placements, placements,
virtual_placement, set_default_colors, colors, scroll, scroll_offset, cell, cols, rows, size, history_len}, RowExtras,
Cell/Color/NamedColor/Rgb/Flags, ImagePlacement, ImageHandle, ImageUpload, placeholder_at,
strip_placeholders, PLACEHOLDER.
-->

# Rendering

The engine draws nothing. It keeps the screen in a form that is cheap to copy from and tells you which
rows changed. This chapter is about turning that into a frame.

## One frame

```text
lock the terminal
  ask what changed (damage)
  copy only those rows into your own reusable buffers
  copy the cursor, the selection range, the placements
unlock
draw from the copies
```

The copy is the only part that needs the terminal, and it is the part the engine is built to make
cheap. Do the shaping, the glyph lookups and the GPU work after the lock is released.

### Damage

The engine marks a row dirty when anything changes it. `take_damage` reports which rows of the visible
view changed since the last call and clears the marks. It also tells you when *everything* changed: the
view moved (you scrolled into the history), the size changed, or the screen was cleared or swapped. In
that case `rows()` lists every row, so the loop below redraws it all (`is_full()` says so).

```rust,ignore
let mut damage = Damage::new(); // keep it between frames
term.take_damage(&mut damage);

for y in damage.rows() {
    term.copy_row(Line(y as i32 - term.scroll_offset() as i32), &mut rows[y].cells, Some(&mut rows[y].extras));
    rows[y].dirty = true;
}
```

Rules that keep this correct:

- **Keep your buffers between frames.** `copy_row` writes into a `Vec<Cell>` and a `RowExtras` that you
  own; once they have grown to the row's width, copying allocates nothing.
- **A row you did not copy is the same as last time.** Keep your last copy, and keep what you built from
  it (shaped text, a texture).
- **Damage is per terminal, not per consumer.** `take_damage` clears the marks. If two renderers read the
  same terminal, one of them gets nothing. Have one reader fan out.
- **Output while scrolled back.** The view keeps showing the same text while new output arrives below
  it, and the damage you get is for the rows you see.
- **The cursor is not a row.** `cursor_changed()` says when its place, visibility or look changed; redraw the
  rows it left and arrived on (read `cursor()` for where). `colors_changed()` says when a program set or
  reset palette entries (OSC 4, 10, 11, 12): redraw everything if you draw with `colors()`.

`copy_viewport` does the whole loop for the visible rows (it copies the changed ones into a
`Vec<ViewRow>` and tells you which in each `changed` field) and is the shortest way to a first renderer.

## Rows, cells and lines

`Line(0)` is the top row of the live screen, history is negative (`concepts.md`). When the view is
scrolled back, the top visible row is `Line(-scroll_offset)`. `scroll` moves the view, and
`scroll_offset` tells where it is. `cell(point)` reads a single cell.

A row copy gives you `Cell`s (character, `fg`, `bg`, `flags`) plus a `RowExtras` with the rare things.
Rows also carry:

- whether the row **continues in the next** (a soft wrap). The copy sets `WRAPLINE` on the row's last
  cell, and `is_wrapped(line)` answers it directly. Use it for text extraction, to show a wrap marker,
  or to decide where a link can continue.
- a **shell mark** (OSC 133): which command block the row belongs to, and whether it is prompt, command
  or output. A host uses them for "jump to previous prompt" or a gutter.

## Wide characters and spacers

A wide character is two cells: the first has `WIDE_CHAR` and the character; the second has
`WIDE_CHAR_SPACER` and holds nothing. Draw the glyph once, over both cells, and skip the spacer. A
`LEADING_WIDE_CHAR_SPACER` at the end of a row is a blank left because a wide character did not fit; draw
the background and nothing else.

A cell's zero-width characters (`extras.zerowidth(col)`) are drawn on top of its character, in the same
cell. They are in print order. A cluster of an emoji with a variation selector and a joiner sequence is a
base character plus zero-width characters; shape the base and its marks as one run, not one by one. At
most 32 marks are kept per cell; the rest are dropped.

Do not draw a character the width tables do not agree with. The engine counts a cell as wide by
`unicode::width` and the clustering rules of mode 2027. If your font draws a glyph at a different width,
the glyph needs to be scaled or clipped, not the grid shifted.

## The cursor

`Terminal::cursor()` gives the cursor's position, whether the program shows it (`DECTCEM`), the shape and
blink the program chose (DECSCUSR, `None` when it left that to you), and whether it stands on a wide
character. The position is on the first cell of a wide character, so draw the block two cells wide when
`wide` is set.

With a **wrap pending** (`concepts.md`), the cursor is reported on the last column, which is where it is
drawn. `cursor_point` is different: it gives the cell the next character will go into. Use `cursor()` for
drawing.

DECSCUSR 0 hands the choice back to you: shape and blink come back `None`. Keep the user's own setting for
that case.

## Selection

The engine keeps the selection, because it must follow the text. Selection endpoints are pins, so a
selection stays on the text it was made on while output scrolls and while the window is resized.

```rust,ignore
term.start_selection(SelectionType::Simple, point, Side::Left);   // mouse down
term.update_selection(point, Side::Right);                        // drag
if let Some(range) = term.selection_range() { /* highlight these cells */ }
let text = term.selection_text();                                 // copy
term.drop_empty_selection();                                      // mouse up without a drag
```

- `side` is the half of the cell the pointer is on. A drag from the middle of one cell to the middle of
  another includes both cells; a click with no drag selects nothing.
- The kinds are `Simple` (reading order), `Block` (rectangle), `Semantic` (words; a double click) and
  `Lines` (whole logical lines; a triple click). Word breaks come from `word_separators`, which you can
  set, and a click on a bracket selects to its partner.
- The text is what copying should give: soft-wrapped rows are joined, other line ends are newlines, blanks
  at the end of a line are cut, and a wide character is never cut in half. Image placeholder cells are
  left out.
- The selection ends when the screen switches between main and alternate, and when the row of an end
  is dropped from the history.
- The range is in grid coordinates (`Line(0)` is the live top). Add the view offset yourself to draw.

For other uses of the text (search, "copy last output"), `text_between`, `copy_text` and the pin-based
span calls give you the text of a range and anchors that survive reflow. Search belongs to the host: the
engine hands out text with a map back to cells, and you run the matching on your own thread, outside
the lock.

## Hyperlinks

OSC 8 links are on the cells a program printed while the link was open. You get them per row:

```rust,ignore
for (columns, link) in rows[y].extras.links() {
    // columns: Range<usize>; link.uri (an Arc<str>), link.id (a LinkId, which derefs to str)
}
let link = rows[y].extras.hyperlink(col); // for a hover test
```

A link is a run: one entry for a link across 80 cells. `link_span(point)` tells how far a link reaches
from a point, following it over soft-wrapped rows (up to 16 rows each way), so you can underline the
whole link when the pointer is on any part of it.

A link without an `id` of its own gets a generated one, so two such links are never joined by hover.
Cells with the same `id` and the same target belong together. The target is text a program chose:
show it before you open it (`security.md`). Control characters are refused by the engine, and the
engine's OSC cap limits a target to about 8 KiB.

## Pictures

The engine knows where pictures go. The host has the pixels. The flow:

1. A program sends a picture (kitty graphics, Sixel or an iTerm2 inline image). The engine queues
   `Event::ImageData(upload)` with the bytes still encoded: PNG, a file in some format your decoder
   reads, or raw RGB/RGBA (optionally zlib-compressed). Sixel is different: the engine decodes it
   itself and hands you RGBA.
2. You decode it, keep the pixels under `upload.image` (an `ImageHandle`) and call
   `image_decoded(handle, Ok(()))`, or `Err(ImageError::..)` if you cannot. Call it exactly once.
3. Each frame, if `has_placements()` is true, call `placements(&mut out)`. You get the placements that
   touch the view, lowest `z` first and the oldest first among equals, which is the draw order. A
   negative `z` goes under the text. This allocates nothing once `out` has grown, and nothing at all
   when no picture is placed.
4. When you receive `Event::ImageFree(handle)`, drop the pixels.

A placement says where the picture's top-left cell is (`line`, `column`), how many cells it covers, which
part of the source to show, pixel offsets inside the first cell and how to size it (`Fill`, `Fit` or
`Pixels`). A tall picture may be partly scrolled into the history; clip it to your view. The cells under a
picture are ordinary cells: text printed over a picture leaves it, and an erase that covers a cell of it
deletes it.

**Placeholders.** kitty can also draw a picture with text: the program prints U+10EEEE
(`PLACEHOLDER`) in cells, and diacritics and colours say which tile of which picture each cell shows. Those
are ordinary text cells, so they scroll, wrap and reflow. Draw each placeholder cell you meet as its tile
(`placeholder_at` decodes a cell; `virtual_placement` says how the picture is sized). No font shows the
character itself. `strip_placeholders` removes them from text you copy.

**Cell size.** Tell the engine the size of a cell in pixels (`cell_size`, and again if the font changes).
It needs it to decide how many cells a picture covers.

**Budgets.** The engine keeps at most 1,024 pictures per screen, 128 MiB as RGBA, evicting the least
recently used (pictures with no placement first), at most 4,096 placements per screen, and at most
256 MiB of uploads waiting for you to decode. A picture bigger than 10,000 pixels a side or 16 million
pixels is refused. Your decoder should enforce its own limits as well: a small PNG can decode to a very
large bitmap, and a zlib bomb is the host's to refuse.

## Colours

A cell's colour is a `Color`:

- `Named`: the default foreground, default background and cursor colour, and the 16 ANSI names.
- `Indexed(n)`: an entry of the 256-colour palette (the 6x6x6 cube and the grey ramp are xterm's).
- `Rgb`: a true colour.

Resolve them through **your** theme. The 16 ANSI names are slots 0 to 15 of the palette, and the three
defaults are slots 256 to 258. Do not draw `Named(Black)` as `#000000` unconditionally: that is your
theme's choice. Flags like `BOLD` or `DIM` do not change the colour in the data; whether bold means a
brighter colour is the renderer's choice, as is what `INVERSE` and `HIDDEN` look like.

### Programs that change the palette

OSC 4, 10, 11, 12 and their resets let a program override palette entries. The engine keeps those
overrides per terminal and exposes them with `colors()`: a table of 259 slots, each `None` ("use the
theme") or `Some(rgb)`. Apply it when you resolve a colour, and treat a change as damage for the whole
screen, since any cell may use that entry. RIS and the reset sequences (OSC 104, 110 to 112) clear them.

For the other direction, tell the engine what your theme's colours are (`set_default_colors`, 259
entries). The engine answers a program's colour query (`OSC 4;n;?`, `OSC 10;?`, `OSC 11;?`) with a colour
the program set, or else with yours. Programs use this to find out whether the terminal is dark or light.
Without a call the engine answers with xterm's palette, a light-grey foreground and a black background.

The engine does not draw the special colours (OSC 5, bold/underline/blink/reverse/italic) or the extra
dynamic colours (OSC 13 to 19). It keeps and answers them, and draws nothing from them.

## Synchronized output and bells

`synchronized_output()` is true while a program is between `CSI ? 2026 h` and `l`. See `embedding.md` for
how to treat it. `Event::Bell` is yours to ring, flash or ignore. Many bells in a row come as one event.
