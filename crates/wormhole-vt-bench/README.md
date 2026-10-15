# wormhole-vt-bench

The tools that test and measure Wormhole's terminal engine (`crates/wormhole-vt`). They talk to an engine through one trait, `Engine`
(`src/engine.rs`): feed bytes, resize, read rows, cells, cursor, history, title, alternate screen,
and take the replies the terminal sent. An engine is one file in `src/engines/` and one line in its
registry (`for_each_engine`, `visit_engine`, `ENGINE_NAMES`).

| tool | what | run |
|------|------|-----|
| conformance suite | `.vtt` cases in `crates/wormhole-vt/tests/conformance/` (format: its `README.md`), every case fed whole and one byte at a time | `cargo test -p wormhole-vt-bench --test conformance` |
| differential tests | the same bytes to two engines (here the engine against itself, cut in pieces; against other terminals' engines: `tools/vt-compare`), compare screen, cells, cursor, replies, history; random streams that lean toward valid sequences; a minimizer that cuts a failing stream down | `cargo test -p wormhole-vt-bench --test differential` |
| corpus | deterministic generators of realistic streams (`src/corpus/`), a test pins their checksums | used by the next two |
| bisimulation | the repaint writer's property: an engine, and a replica made from its repaint and fed the same bytes after, stay equal. Every `.vtt` case cut at every byte, every corpus at many cuts (inside escape sequences and UTF-8 characters too), with and without a resize at the cut; the whole state is compared (`Terminal::dump_state`) and a mismatch names the first field. What a repaint says it left out (`Gaps`) is masked and has a case that shows it differs | `cargo test -p wormhole-vt-bench --test bisimulation --test bisimulation_gaps` (`BISIM_CUTS`, `BISIM_SIZE`, `BISIM_FILTER`, `BISIM_ALL`) |
| `repaint-bench` | what a repaint costs: write time, size and a replica's parse time at 10,000 and 200,000 lines | `cargo run --release -p wormhole-vt-bench --bin repaint-bench` |
| `vtbench` | MB/s and allocations per MB, per engine and corpus (and every recording in `tools/vt-corpus`) | `cargo run --release -p wormhole-vt-bench --bin vtbench` (`--help`, `--json`) |
| `ucs-host` | the engine behind two pipes (bytes in, replies out), so `tools/vt-compare/ucs-headless.py` can run ucs-detect against it through a pseudo-terminal in a second, with no window | `tools/vt-compare/ucs-headless.py` (`--clusters on|off`, `-- --all`) |
| `scripts/vt-record.sh` | records a real program's output into `tools/vt-corpus/` | by hand, from a terminal |

## Adding an engine

Other terminals' engines (alacritty, Ghostty, WezTerm, vt100) are adapters in `tools/vt-compare`,
its own workspace, which also has the parity tests against alacritty (random streams, selection
and copying) and the parser benchmark against `vte`. The same steps apply there.

1. `src/engines/<name>.rs`: a type implementing `Engine`, following the rules in the trait's
   documentation (everything fed is applied when `feed` returns, replies in the order they were
   made, colors and window size answered with the fixed values, `row_text` without trailing
   blanks).
2. Register it in `src/engines/mod.rs` (`ENGINE_NAMES`, `for_each_engine`, `visit_engine`).
3. `cargo test -p wormhole-vt-bench --test conformance`: its failures are what is missing. Do not
   write `known-fail:` lines for the new engine; it is expected to pass every case.
4. Differential: `diff::fuzz::<WormholeEngine, YourEngine>(...)` and `diff::compare`; the
   `Compare` switches (text, cells, cursor, replies, title, alt_screen, scrollback) narrow it to
   what the engine does. Streams with UTF-8 text should be cut at random places too
   (`Split::Random`).
5. `vtbench` shows it next to the others (the first engine in the list is the one the rest are
   compared to).

## Dependencies

`bitflags` (cell attributes),
`serde_json` (vtbench's `--json`). The crate depends on the engines and never the other way round:
`wormhole-vt` must not depend on its tools.
