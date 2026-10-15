# Fuzzing wormhole-vt

Coverage-guided fuzzing with `cargo fuzz` (libFuzzer, address sanitizer, overflow checks on). It needs
a nightly toolchain and is run by hand: it is not part of `scripts/check.sh` or CI, and the crate is
its own workspace so that `cargo check --workspace` never sees it.

```sh
rustup toolchain install nightly --profile minimal   # once
cargo install cargo-fuzz                             # once
scripts/fuzz.sh seeds                                # the starting corpus (not committed, see below)
scripts/fuzz.sh run -m 20                            # every target, 20 minutes, 4 processes in all
scripts/fuzz.sh run term graphics -m 30 -j 2         # chosen targets
scripts/fuzz.sh repro term crates/wormhole-vt/fuzz/artifacts/term/crash-...
scripts/fuzz.sh min                                  # shrink the corpus (cmin)
```

The script runs at low priority and never starts more than 4 processes. A finding is saved as
`fuzz/artifacts/<target>/crash-*` (or `timeout-*`, `oom-*`, `slow-unit-*`): reproduce it with `repro`, fix
the engine from the spec, and turn the input into a regular test next to the code it broke (a fixed
byte string; the test names the input's origin), then delete the artifact.

## Targets

| target | what is fed | what must hold |
|--------|-------------|----------------|
| `parser` | any bytes, cut into `advance` calls where the first bytes say; with a recording handler and a silent one, 7-bit and 8-bit controls | no panic or hang; the events are the same whole, cut, and byte by byte; what a handler is given is well formed (parameters, intermediates, string payloads) |
| `split` | any bytes into two terminals, one whole and one cut | the same screen, history, cursor, title, mode, replies and events |
| `term` | a script (below): a terminal of any size and history, the program's bytes and the host's calls mixed | no panic, hang or runaway memory; `Terminal::check_invariants` after every step |
| `graphics` | frames of kitty APC commands, Sixel DCS, iTerm2 OSC 1337 files and plain text, around arbitrary bytes (a frame's payload can be base64 encoded by the target so that real pictures get through), the host answering every picture | the same as `term`, and the pictures stay within their budgets |

A **script** (`src/lib.rs`) is a 3-byte header (columns, rows, history size and flags: screen checksum
answered, mode 2027 on, cell size known) and then the stream, in which `0xFF` and an opcode is a call of
the host: resize, scroll the view, select (all four kinds), update or take the selection, copy text for a
search and pin the matches, copy the viewport and the damage, text between points, pins, clearing
history, changing the history limit, the snapshot reads, answering a picture the engine never asked for.
Points are mostly on the screen, sometimes in the history, now and then wild. `0xFF 0xFF` is a literal
`0xFF`. A byte string of plain escape sequences is a valid script, so the `.vtt` cases and the corpora
seed it as they are.

`escape.dict` is a dictionary of introducers, terminators and the picture protocols' keys.

## The corpus

`scripts/fuzz.sh seeds` runs `cargo run -p wormhole-vt-bench --bin fuzz-seeds`, which writes about 3,000
seeds into `fuzz/corpus/<target>/` (git-ignored): every `.vtt` case's input and resizes, windows of the
generated corpora and, when `tools/vt-corpus` has them, of the recorded sessions. The corpus the
fuzzer grows from there is not committed (it is rebuilt in minutes); `scripts/fuzz.sh min` shrinks it.

The in-tree tests run the same checks on generated streams on every `cargo test`
(`src/parser/tests/property.rs`, `src/term/tests/hostile.rs`, the grid's random tests).
