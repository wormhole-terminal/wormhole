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

#[cfg(test)]
mod tests {
    use std::hash::Hasher;

    use super::OffsetHasher;

    #[test]
    fn an_offset_is_one_multiply_and_its_high_half_is_folded_into_the_low() {
        let mut hasher = OffsetHasher::default();
        hasher.write_u32(1);
        let product = 0x9E37_79B9_7F4A_7C15_u64;
        assert_eq!(hasher.finish(), product ^ (product >> 32));
        hasher.write_u32(7);
        let product = 7_u64.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        assert_eq!(hasher.finish(), product ^ (product >> 32), "the offset replaces what was hashed");
    }

    #[test]
    fn bytes_are_hashed_one_at_a_time_in_order() {
        let (mut ab, mut ba) = (OffsetHasher::default(), OffsetHasher::default());
        ab.write(b"ab");
        ba.write(b"ba");
        let mut state = 0_u64;
        for byte in *b"ab" {
            state = (state ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
        }
        assert_eq!(ab.finish(), state ^ (state >> 32));
        assert_ne!(ab.finish(), ba.finish());
    }
}
