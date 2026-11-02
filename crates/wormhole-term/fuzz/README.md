# Fuzzing wormhole-term's checkpoint reader

The same setup as `crates/wormhole-wire/fuzz`: `cargo fuzz`, nightly, by hand, run through `scripts/fuzz.sh`, not
part of `scripts/check.sh` or CI, and its own workspace.

```sh
scripts/fuzz.sh run checkpoint_read -m 10
scripts/fuzz.sh repro checkpoint_read crates/wormhole-term/fuzz/artifacts/checkpoint_read/crash-...
```

A finding is saved as `fuzz/artifacts/<target>/crash-*`: turn the input into a fixed byte string in
`src/checkpoint/tests.rs` (named for where it came from), fix the reader, delete the artifact.

| target | what is fed | what must hold |
|--------|-------------|----------------|
| `checkpoint_read` | one byte that splits the rest into a manifest file and a screen file, put in a checkpoint directory in place of a good generation 1 | no panic or hang in the manifest parser, the directory reader or the replay filter; a screen that comes back is one that opened under the key; the filter lets only text, line ends and the escape of an SGR through. The in-tree test `hostile_files_never_panic_the_reader` runs the same properties on mutated files on every `cargo test`. |
