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

- VT52 mode: `CSI ? 2 l` (DECANM reset) makes the terminal read the VT52's sequences (cursor moves,
  `ESC Y` direct addressing, `ESC F` and `ESC G` graphics, `ESC H`, `I`, `J`, `K`, `Z` answered
  `ESC / Z`, `ESC =` and `ESC >`) and `ESC <` leaves it, at VT100 level as the VT510 manual and xterm have it. `Modes::VT52`, and the key encoder sends the
  VT52's keys for it. The parser reads an ESC and one byte only while it lasts
  (`Handler::vt52_mode`).
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
- `Handler::vt52_mode`, `Modes::{VT52, INBAND_RESIZE, MODIFY_OTHER_KEYS, MODIFY_OTHER_KEYS_ALL}`,
  `ModifyOtherKeys`, and the fields `InputModes::{vt52, modify_other_keys}`.
- Double-width and double-height lines: DECDWL (`ESC # 6`), DECDHL (`ESC # 3` and `ESC # 4`) and DECSWL
  (`ESC # 5`). The size is per row and moves with it (scrolling, insert and delete line, the history),
  the cursor and the right margin stop at half the columns, and such a row is never reflowed. The host
  reads it as `LineSize`: `Terminal::line_size`, `ViewRow::size`, `HistoryLine::size`; the repaint writes
  it.
- A front door at the crate root: `Terminal::new(cols, rows)` and `Terminal::builder()` (size,
  scrollback, policy, cell size, palette, word separators, passthrough). `Terminal::feed(bytes)` takes
  program output; the terminal owns its parser.
- `Policy`, one place for every risky switch: window operations, title reports and DECRQCRA, all off by
  default. Title reports (XTWINOPS 20 and 21) are a switch of their own, `Policy::title_report`.
- The `input` module: key encoding (legacy xterm, modifyOtherKeys, the VT52's keys and the kitty keyboard
  protocol), mouse reports (X10, UTF-8, URXVT, SGR and SGR with pixels), bracketed paste with ESC and the
  C1 controls removed, and focus reports, as pure functions of the event and the modes the program set
  (`InputModes`, made from `Modes`).
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
- Documentation: the README is the crate's front page, the `guide` module holds the chapters (concepts,
  embedding, rendering, input, passthrough, security, conformance, tools) with code that `cargo test`
  runs, `#![deny(missing_docs)]` keeps every public item described, and `SECURITY.md` says how to report a
  vulnerability.
- Examples: `quickstart`, `ansi_to_html`, `repaint`, `custom_osc`, `vtdump`, `vtreplay` and the
  benchmarks `profile_feed`, `unicode_bench` and `grid_bench`.
- `api.txt`: the public API as `cargo public-api` lists it, with `scripts/vt-api.sh` to check or update
  it, so an API change is always a visible diff.
- Compile-time checks that `Terminal`, `Builder` and the value types are `Send` and `Sync`, and that the
  error types implement `std::error::Error`.
- The minimum supported Rust version is 1.88.

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
- `Event` and the public enums and option structs are `#[non_exhaustive]`; `Terminal` is `Send` and `Sync`,
  checked at compile time.
- The reading API has plain names: `cols`, `rows`, `history_len`, `scroll_offset`, `scroll` (were
  `columns`, `screen_lines`, `history_size`, `display_offset`, `scroll_display`), `cursor_point` (was
  `grid_cursor`), `block_text` and `BlockPart` (were `content_text` and `Content`); `take_damage` fills a
  `Damage` instead of a `[bool]`.
- The engine no longer names the app: XTVERSION and generated link ids no longer say "Wormhole".
- OSC 1337 that is not an inline image reaches the host capped at 64 KiB (it was 64 MiB).
- Media copy (`CSI Ps i`, `CSI ? Ps i`) is recognised and ignored, the printer controller mode
  (`CSI 5 i` until `CSI 4 i`) included: what follows is drawn. There is no printer, and swallowing the
  data would let a stray `CSI 5 i` silence the terminal for good.
- The crate has its own version, `0.1.0`, `rust-version` and package metadata; it is no longer versioned
  with the Wormhole app.
- `Hyperlink::id` is a `LinkId`, not an `Arc<str>`: it derefs to `str`, and ids of up to 22 bytes (every
  generated one) are stored in the value, so a link costs no allocation for its id. The text of
  addresses is shared through a bounded table, so a link that repeats costs none for its address either.

### Security

- Titles, icon names and folders (OSC 0, 1, 2, 7) are cleaned when they are set: C0 and C1 controls and
  DEL are dropped.
- Title reports no longer follow `window_ops`.
- DECCOLM (`CSI ? 3 h/l`) changes the width only when `Policy::window_ops` allows it, the gate
  DECSCPP and DECSLPP are behind. Without it the mode is recorded, the screen is cleared (unless mode 95
  keeps it), the margins and tab stops are reset and the cursor goes home, but the terminal is not
  resized and `Event::ColumnsRequested` is not sent. A resize reflows the whole history, so a few
  kilobytes of toggles used to freeze a terminal for seconds. `ColumnsRequested` is also not sent when the
  width already is the one asked for.
- The events a host has not taken are held to a byte budget as well as a count: `Limits::events_bytes`
  (ceiling 64 MiB) bounds the text and data of clipboard stores, notifications, icon names and passed-through,
  iTerm2 and program status strings, and a new event past it is dropped whole. By count alone 16,384 stores
  of 8 MiB were allowed to wait.
- DECFRA, DECCRA and DECSERA look a style up once for each run of cells with one look, as DECERA does,
  not once for every cell (about thirty times the work of ECH per cell). REP of an ASCII character past two
  screens' worth prints two screens and the remainder that keeps the column: the screen and the cursor
  end where the whole count would have left them, the history gains fewer identical lines.
- The table that shares cell styles hashes with a random seed per page and a folded multiply, so colors
  picked to land in one slot of its index no longer make every lookup walk a long run.
- Replies are kept or dropped as a whole by one queue (`reports::queue`), decided when the reply is
  complete: the piece of a reply that ended it (`ESC \`) was dropped alone when the queue filled up
  between its pieces, leaving a string the program never saw end. The replies held behind a picture's
  answer follow the same cap and the one-byte controls of S8C1T as the others (they bypassed both once
  the picture was decoded), and the queue is never past its cap: a reply that does not fit is dropped,
  unless nothing else waits.
- The cursor and its delayed wrap are brought in line with the row and the margins after a soft reset (a
  wrap due at a right margin that was removed), a scroll that puts another row under them, a swap of the
  screens and a picture placed on a double-width row (`Terminal::check_invariants` failed after these
  four streams).
- Documented guarantees: input never panics or grows memory without bound, replies are built from the
  engine's state, title reports and window operations are off unless the host allows them, and clipboard
  and notifications are requests to the host. See `docs/security.md`.
