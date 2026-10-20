<!--
Assumed API names: Terminal::{new, builder, feed}, Policy, input, ext, Damage, Pin/Span (see README.md and
docs/*.md headers). The "Changed" list repeats decisions D-4 to D-16 of the library spec; trim it to what
phases 1-3 actually shipped before release.
-->

# Changelog

All notable changes to `wormhole-vt` are listed here. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, a minor version may change the API;
each such change is listed under **Changed** or **Removed**.

The engine has been part of the Wormhole app since that app's 1.0.0. This is the first release as a
library of its own.

## [Unreleased]

First public release (0.1.0).

### Added

- A front door at the crate root: `Terminal::new(cols, rows)` and `Terminal::builder()` (size,
  scrollback, policy, cell size, palette, word separators, extensions). `Terminal::feed(bytes)` takes
  program output; the terminal owns its parser.
- `Policy`, one place for every risky switch: window operations (and with them the title reports) and
  DECRQCRA, all off by default.
- The `input` module: key encoding (legacy xterm and the kitty keyboard protocol), mouse reports (X10,
  UTF-8 and SGR), bracketed paste with the end marker removed, and focus reports, as pure functions of the
  event and the terminal's modes.
- The `ext` module: extensions that claim OSC, DCS and APC sequences and private CSI finals, with payload
  caps, `Handled` or `Pass`, and a narrow context for replies and events.
- `Damage`, a reusable description of what changed since the last look (rows, the cursor, the colours a
  program set), and a read API for cells and rows: `Terminal::cell`, `size`.
- Pins and spans that follow text through scrolling and reflow.
- A guide in rustdoc (concepts, embedding, rendering, input, extensions, security, conformance), a README,
  and `SECURITY.md`.

### Changed

- The engine is licensed under the MIT licence (the rest of the Wormhole repository keeps its own).
- One `Cell`, one `Extent` and one `Modes` type; the second cell type and the `TermMode` bit set are gone.
- `grid` is private. What a host needs from it has a public home.
- `Event` and the public enums and option structs are `#[non_exhaustive]`; `Terminal` is `Send`, checked
  at compile time.
- The reading API has plain names: `cols`, `rows`, `history_len`, `scroll_offset`, `scroll` (were
  `columns`, `screen_lines`, `history_size`, `display_offset`, `scroll_display`), `cursor_point` (was
  `grid_cursor`), `block_text` and `BlockPart` (were `content_text` and `Content`); `take_damage` fills a
  `Damage` instead of a `[bool]`.
- The engine no longer names the app: XTVERSION and generated link ids no longer say "Wormhole".

### Security

- Documented guarantees: input never panics or grows memory without bound, replies are built from the
  engine's state, title reports and window operations are off unless the host allows them, and clipboard
  and notifications are requests to the host. See `docs/security.md`.
