---
paths:
  - "crates/wormhole/src/terminal_view/render/**"
  - "crates/wormhole-core/src/terminal/pty_filter/**"
  - "crates/wormhole-core/src/terminal/snapshot.rs"
  - "Cargo.toml"
  - "Cargo.lock"
---

# Terminal rendering and PTY speed

The terminal draws a full screen of changing truecolor cells in 1–3 ms (it took 30–40 ms before
October 2026) and the OSC tap reads colored output at about 2 GB/s. Keep them:

- **Paint the grid's cells inside `window.paint_layer(…)`.** A quad or glyph painted outside a
  layer goes into GPUI's bounds tree, one by one; a screen of colored cells is thousands of them.
  `tests/design_rules.rs` fails on a per-cell paint in a `cache.rows()` loop outside a layer. Keep
  the layers' order: backgrounds, images, box quads and strokes, glyphs, lines through and under
  the text; the cursor's character, link underlines and find ticks after them.
- **Shape only what is new.** Rows are rebuilt only where the snapshot says they changed
  (`RowCache`), words are shaped once by text and fonts (`WordCache`), and characters that never
  depend on their neighbours (`stands_alone`: CJK, kana, Hangul, braille) once each. Never shape
  per frame what didn't change, and don't add a character to `stands_alone` that a font can
  shape contextually (Latin, Arabic, marks, emoji).
- **The OSC tap skips text and complete CSI sequences** while its parser is in the ground state
  (module docs of `pty_filter/osc_tap.rs`). Anything that changes what it skips must keep
  `skipping_text_and_csi_changes_nothing_the_tap_reports_or_hands_over` passing.
- The counting tests in `render/guards.rs`, `render/bench.rs`
  (`frames_stay_within_their_allocations`) and the tap's `colored_output_and_plain_text_mostly_bypass_the_parser`
  are the guard: if one fails, the change made frames do more work. Don't raise a limit to get
  green; find out what got slower, and if it is worth it say so in the commit.

After a change here, and after any GPUI update, run `scripts/perf.sh` on a Mac: it builds a
release, measures draw time per frame and main-thread load under termbench's workloads, and fails
against the baseline in `tools/bench/baselines/`. To look deeper: `sample <pid> 5` on an isolated
instance (see the `devctl` skill), devctl `frames <n>` (draw time, any window state) and `paints`
(frames drawn on screen). `paints` means nothing while the window can't be seen (display asleep,
covered): macOS throttles it to ~20 per second.
