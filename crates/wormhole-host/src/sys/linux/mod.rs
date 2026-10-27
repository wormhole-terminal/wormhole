//! Linux's half of the edge, where the host has something macOS has none of an equivalent for. Nothing
//! builds it yet (macOS is the only target), so the stand-ins answer as the host did here before the edge
//! existed: no QoS, no priority policy, no code signatures.

pub(super) mod process;
