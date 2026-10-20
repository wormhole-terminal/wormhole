<!--
Assumed API names: Terminal::{new, builder, feed}, Policy, input, passthrough, Damage, Pin/Span (see README.md and
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

- In-band resize notifications (DECSET 2048): the program is told `CSI 48 ; rows ; cols ; height ; width t`
  when it sets the mode and after every resize, with the pixels from the cell size the host gave.
- xterm's modifyOtherKeys: `CSI > 4 ; Pv m` sets level 0, 1 or 2, `CSI ? 4 m` asks, `Modes::modify_other_keys`
  and `InputModes::modify_other_keys` carry it, and the legacy key encoder writes
  `CSI 27 ; modifiers ; code ~` as xterm's table of modified keys does. DECSTR and RIS reset it.
- XTPUSHSGR, XTPOPSGR and XTREPORTSGR (`CSI Pm # {`, `CSI # }`, `CSI Pt ; Pl ; Pb ; Pr # |`; the aliases
  `CSI # p` and `CSI # q` too): a stack of 10 pens, and the rendition a rectangle has in common.
- XTPUSHCOLORS, XTPOPCOLORS and XTREPORTCOLORS (`CSI # P`, `CSI # Q`, `CSI # R`): a stack of 10 palettes
  the program set, with slots; a pop is announced with `Event::ColorsChanged`. The repaint writes both
  stacks.
- A front door at the crate root: `Terminal::new(cols, rows)` and `Terminal::builder()` (size,
  scrollback, policy, cell size, palette, word separators, passthrough). `Terminal::feed(bytes)` takes
  program output; the terminal owns its parser.
- `Policy`, one place for every risky switch: window operations, title reports and DECRQCRA, all off by
  default. Title reports (XTWINOPS 20 and 21) are a switch of their own, `Policy::title_report`.
- The `input` module: key encoding (legacy xterm and the kitty keyboard protocol), mouse reports (X10,
  UTF-8 and SGR), bracketed paste with the end marker removed, and focus reports, as pure functions of the
  event and the terminal's modes.
- The `passthrough` module: ask for OSC numbers, DCS and APC introducers the engine does not know and get
  them whole as `Event::Osc`, `Event::Dcs` and `Event::Apc`, with a payload limit and the `replies_before`
  mark a host answers at, so replies keep their order.
- `Damage`, a reusable description of what changed since the last look (rows, the cursor, the colours a
  program set), and a read API for cells and rows: `Terminal::cell`, `size`.
- Pins and spans that follow text through scrolling and reflow.
- `Event::HistoryDropped`: the history lines that leave the engine (scrolled past the limit, cleared by
  ED 3, a full reset or `clear_history`, or cut because the limit fell) handed to the host in order, in
  batches with their reason (`DropReason`), each row with its cells, colours, hyperlinks and wrap flag
  (`HistoryLine`). Opt-in with `Builder::archive_history` or `Terminal::set_archive_history`, free when
  off. The lines wait for the host in a bounded backlog; past it the newest are counted in
  `DroppedHistory::skipped`.
- `Limits`, with `Builder::limits`, `Terminal::set_limits` and `Terminal::limits`: the host lowers the
  caps on pictures (bytes, count, one transmission, queued uploads), Sixel data, OSC strings (small ones,
  the clipboard, pictures, the rest of OSC 1337), titles, folders, hyperlink targets, the event and reply
  queues and the dropped-history backlog. Each value can only go below the built-in ceiling, which stays
  the default; pictures held over a lowered count or budget are evicted at once.
- `Event::ModesChanged` and `Event::ColorsChanged`: a mode a host acts on (the mouse, bracketed paste,
  the alternate screen, focus reports, the application cursor keys and keypad, DECBKM, the kitty
  keyboard flags, reverse video, synchronized output, the cursor's look, the DEC locator) or the colours a
  program set changed. At most one of each per call of `Terminal::events`, with no payload and no cost
  while a program prints.
- Light and dark reports: `Builder::color_scheme` and `Terminal::set_color_scheme(ColorScheme)`. `CSI ? 996 n`
  is answered `CSI ? 997 ; 1 n` (dark) or `; 2 n` (light), and a change is reported unasked while mode 2031
  is set (Contour's colour palette update notifications). Not answered until the host says.
- `Event::PointerShape` for OSC 22, as a CSS cursor keyword (empty: the host's own pointer), and
  `Event::UserVar { name, value }` for iTerm2's `SetUserVar`, base64 decoded and capped (256-byte names,
  64 KiB values, 1 MiB queued). The rest of OSC 1337 stays `Event::Iterm`.
- A guide in rustdoc (concepts, embedding, rendering, input, extensions, security, conformance), a README,
  and `SECURITY.md`.

### Changed

- The engine is licensed under the MIT licence (the rest of the Wormhole repository keeps its own).
- One `Cell`, one `Extent` and one `Modes` type; the second cell type and the `TermMode` bit set are gone.
  `Modes` (a bit set, every flag documented with its number) answers `mouse_tracking()`,
  `mouse_encoding()` and `kitty_flags()`, and `InputModes` is made from it with `From<Modes>` and
  `From<&Modes>`, which now carries the application keypad, X10 tracking, URXVT, SGR-pixels and DECBKM
  too. `Terminal::mode()` is gone; `Terminal::modes()` adds `Modes::LOCATOR` while the DEC locator is on.
- DECBKM (mode 67) starts reset, as xterm-256color's terminfo (`kbs=^?`) and every other terminal a
  shell runs in has it, so the backarrow key sends DEL until a program sets it; `InputModes::backarrow_bs`
  says which.
- `grid` is private. What a host needs from it has a public home.
- `Event` and the public enums and option structs are `#[non_exhaustive]`; `Terminal` is `Send`, checked
  at compile time.
- The reading API has plain names: `cols`, `rows`, `history_len`, `scroll_offset`, `scroll` (were
  `columns`, `screen_lines`, `history_size`, `display_offset`, `scroll_display`), `cursor_point` (was
  `grid_cursor`), `block_text` and `BlockPart` (were `content_text` and `Content`); `take_damage` fills a
  `Damage` instead of a `[bool]`.
- The engine no longer names the app: XTVERSION and generated link ids no longer say "Wormhole".
- OSC 1337 that is not an inline image reaches the host capped at 64 KiB (it was 64 MiB).
- `Hyperlink::id` is a `LinkId`, not an `Arc<str>`: it derefs to `str`, and ids of up to 22 bytes (every
  generated one) are stored in the value, so a link costs no allocation for its id. The text of
  addresses is shared through a bounded table, so a link that repeats costs none for its address either.

### Security

- Titles, icon names and folders (OSC 0, 1, 2, 7) are cleaned when they are set: C0 and C1 controls and
  DEL are dropped.
- Title reports no longer follow `window_ops`.
- Documented guarantees: input never panics or grows memory without bound, replies are built from the
  engine's state, title reports and window operations are off unless the host allows them, and clipboard
  and notifications are requests to the host. See `docs/security.md`.
