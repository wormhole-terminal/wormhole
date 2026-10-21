//! The hash map shape of the page's side tables: a cell's offset in the page is the key.
//!
//! The marks table (`graphemes`) is keyed by the cell's offset (`slot * columns + column`), which is
//! stable while rows are moved by relinking, and is rare enough that a hash map is the right shape:
//! a page of plain text has none, a page of emoji has many marks.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Hashes the `u32` cell offsets that key the tables: one multiply, where SipHash would cost more
/// than the lookup is worth for a key nobody chooses adversarially.
#[derive(Default)]
pub(super) struct OffsetHasher(u64);

impl Hasher for OffsetHasher {
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 32)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ byte as u64).wrapping_mul(0x0100_0000_01B3);
        }
    }

    fn write_u32(&mut self, n: u32) {
        self.0 = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

pub(super) type ByOffset<V> = HashMap<u32, V, BuildHasherDefault<OffsetHasher>>;
