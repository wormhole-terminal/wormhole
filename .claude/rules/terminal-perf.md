---
paths:
  - "crates/wormhole/src/terminal_view/render/**"
  - "crates/wormhole-vt/src/**"
  - "crates/wormhole-core/src/terminal/pty/**"
  - "crates/wormhole-core/src/terminal/engine/**"
  - "crates/wormhole-core/src/terminal/snapshot.rs"
  - "Cargo.toml"
  - "Cargo.lock"
---

# Terminal rendering and PTY speed

The terminal draws a full screen of changing truecolor cells in 1–3 ms (it took 30–40 ms before
October 2026); the engine (`wormhole-vt`) reads output several times faster than alacritty did, and
the app leads the other terminals overall in `scripts/terminal-compare.sh`. Keep them:

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
- **The engine's hot paths allocate nothing**: printing, scrolling, styles and links reuse the
  grid's pages and tables. `crates/wormhole-core/tests/pty_allocations.rs`
  (`reading_and_parsing_allocates_nothing`) pins the reader thread at zero allocations in floods,
  and `vtbench` (`crates/wormhole-vt-bench`) shows allocations per MB for each corpus.
- **The PTY reader re-reads an empty master for 50 µs after output** before it sleeps
  (`terminal/pty/reader.rs`): macOS hands the reader 1 KiB at a time, and an engine faster than the
  writer would otherwise sleep and be woken for every refill. Measure before changing it.
- The counting tests in `render/guards.rs`, `render/bench.rs`
  (`frames_stay_within_their_allocations`) and `pty_allocations.rs` are the guard: if one fails,
  the change made frames or reads do more work. Don't raise a limit to get green; find out what got slower, and if it is worth it say so in the commit.

After a change here, and after any GPUI update, run `scripts/perf.sh` on a Mac: it builds a
release, measures draw time per frame and main-thread load under ttystorm's workloads, and fails
against the baseline in `tools/bench/baselines/`. The engine alone: `vtbench`, and against other
engines `tools/vt-compare` (`../repo-docs/docs/performance.md`). To look deeper: `sample <pid> 5`
on an isolated instance (see the `devctl` skill), devctl `frames <n>` (draw time, any window state) and `paints`
(frames drawn on screen). `paints` means nothing while the window can't be seen (display asleep,
covered): macOS throttles it to ~20 per second.
