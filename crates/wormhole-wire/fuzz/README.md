# Fuzzing wormhole-wire

Same setup as the engine's (`../../wormhole-vt/fuzz/README.md`): `cargo fuzz`, nightly, by hand, run through
`scripts/fuzz.sh` (which knows both crates), not part of `scripts/check.sh` or CI, and its own workspace.

```sh
scripts/fuzz.sh seeds                                   # the starting corpus: one input per sample message
scripts/fuzz.sh run wire_frame wire_message -m 10       # both targets, 10 minutes
scripts/fuzz.sh repro wire_frame crates/wormhole-wire/fuzz/artifacts/wire_frame/crash-...
```

A finding is saved as `fuzz/artifacts/<target>/crash-*`: turn the input into a fixed byte string in
`tests/hostile.rs` (named for where it came from), fix the decoder, delete the artifact.

## Targets

| target | what is fed | what must hold |
|--------|-------------|----------------|
| `wire_frame` | two control bytes (how the stream is cut, whether the pre-handshake size limit applies) and then any bytes, into the stream `Decoder` | no panic or hang; the buffer never passes one maximum frame plus a read; the same frames and errors come out whole and cut at any size; every decoded message encodes again and decodes to itself (the only message the encoder may refuse is one holding an event kind from a newer minor) |
| `wire_message` | a type id and then a payload, straight into `Message::decode` | the same, without the framing in the way, so the payload parsers get all the coverage |

`wire.dict` is a dictionary of the magic, the type ids and the length boundaries. The in-tree tests run
the same properties on generated and mutated streams on every `cargo test` (`tests/roundtrip.rs`,
`tests/hostile.rs`).
