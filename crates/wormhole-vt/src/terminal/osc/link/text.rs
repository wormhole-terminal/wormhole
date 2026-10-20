//! The text of hyperlinks, shared: one allocation per distinct address and id, however often a
//! program names them.
//!
//! A hyperlink holds its address as an `Arc<str>`, so that the cells under it, the pages that hold
//! them and the host that reads them all share one copy; its id is stored in the value when it is
//! short (see [`LinkId`]), which is nearly always, and shared like the address when it is not.
//! Making those strings for every OSC 8 was the allocation a link cost, and a program that lists
//! files or prints build errors says the same few hundred addresses again and again. So the
//! terminal keeps the strings it made in two bounded tables keyed by content, and a link whose
//! text is in them costs two lookups and two reference counts. A link without an id of its own
//! gets `link-N`, which is stored in the id and allocates nothing.
//!
//! Both tables are bounded in entries and bytes, and an entry that cells still hold is never
//! dropped to make room, so a program cannot make the tables grow by naming a million addresses
//! (it gets new strings that are not kept).

use std::collections::HashSet;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::sync::Arc;

use crate::types::LinkId;

/// The most addresses the table keeps, and the most bytes of them: 16,384 cover a listing of a big
/// repository, and the bytes cap is for addresses of kilobytes. At most this much memory is held
/// for text that no cell is under; text that cells are under is theirs, not the table's.
const URIS: Limits = Limits { entries: 16_384, bytes: 2 * 1024 * 1024 };
/// The same for the ids too long to be stored in a link (a UUID is 36 bytes): rare, so less.
const IDS: Limits = Limits { entries: 1024, bytes: 128 * 1024 };

#[derive(Clone, Copy)]
struct Limits {
    entries: usize,
    bytes: usize,
}

/// Hashes strings with a key chosen at run time, so a program cannot choose strings that collide.
/// One multiply-fold per eight bytes: the tables are looked up for every link, so SipHash's cost on
/// an address of fifty bytes would show.
#[derive(Clone)]
struct Seeded(u64);

struct TextHasher(u64);

impl BuildHasher for Seeded {
    type Hasher = TextHasher;

    fn build_hasher(&self) -> TextHasher {
        TextHasher(self.0)
    }
}

impl TextHasher {
    #[inline]
    fn mix(&mut self, word: u64) {
        let product = u128::from(self.0 ^ word).wrapping_mul(0x9E37_79B9_7F4A_7C15_F39C_C060_5CED_C835);
        self.0 = (product as u64) ^ ((product >> 64) as u64);
    }
}

impl Hasher for TextHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.mix(u64::from_le_bytes(*word));
        }
        let mut last = [0u8; 8];
        last[..rest.len()].copy_from_slice(rest);
        // The length goes in too, so "a" and "a\0" differ.
        self.mix(u64::from_le_bytes(last) ^ ((rest.len() as u64) << 56));
    }

    #[inline]
    fn write_u8(&mut self, byte: u8) {
        self.mix(u64::from(byte));
    }
}

/// A bounded set of strings, by content.
struct Pool {
    limits: Limits,
    strings: HashSet<Arc<str>, Seeded>,
    bytes: usize,
    /// Strings refused for want of room since the pool last looked for ones nobody holds.
    refused: usize,
}

impl Pool {
    fn new(seed: u64, limits: Limits) -> Pool {
        Pool { limits, strings: HashSet::with_hasher(Seeded(seed)), bytes: 0, refused: 0 }
    }

    /// The shared copy of `text`, made (and kept, if there is room) when this is its first sight.
    fn get(&mut self, text: &str) -> Arc<str> {
        if let Some(known) = self.strings.get(text) {
            return known.clone();
        }
        let made: Arc<str> = Arc::from(text);
        if self.has_room_for(text.len()) {
            self.bytes += text.len();
            self.strings.insert(made.clone());
        }
        made
    }

    fn has_room_for(&mut self, len: usize) -> bool {
        let fits =
            |pool: &Pool| pool.strings.len() < pool.limits.entries && pool.bytes + len <= pool.limits.bytes;
        if fits(self) {
            return true;
        }
        // Full: the strings nobody but the pool holds are only a cache and can go. Looking is a
        // pass over the table, so it is done once per eighth of a table of refusals, not for each.
        self.refused += 1;
        if self.refused < self.limits.entries / 8 {
            return false;
        }
        self.refused = 0;
        let mut bytes = 0;
        self.strings.retain(|text| {
            let held = Arc::strong_count(text) > 1;
            if held {
                bytes += text.len();
            }
            held
        });
        self.bytes = bytes;
        fits(self)
    }
}

/// The strings of a terminal's hyperlinks; see the module docs.
pub(in crate::terminal) struct LinkText {
    uris: Pool,
    ids: Pool,
}

impl Default for LinkText {
    fn default() -> LinkText {
        // `RandomState` has the process's random keys; the two tables get different seeds.
        let seed = RandomState::new().hash_one(0x6c69_6e6b_u64);
        LinkText { uris: Pool::new(seed, URIS), ids: Pool::new(seed.rotate_left(32), IDS) }
    }
}

impl LinkText {
    pub(in crate::terminal) fn uri(&mut self, uri: &str) -> Arc<str> {
        self.uris.get(uri)
    }

    /// The id `id` names: stored in the value when short, else the shared copy.
    pub(in crate::terminal) fn id(&mut self, id: &str) -> LinkId {
        if LinkId::is_inline(id) { LinkId::new(id) } else { LinkId::from_shared(self.ids.get(id)) }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{LinkText, URIS};

    #[test]
    fn the_same_text_is_one_allocation() {
        let mut text = LinkText::default();
        let (a, b) = (text.uri("https://example.com/a"), text.uri("https://example.com/a"));
        assert!(Arc::ptr_eq(&a, &b));
        assert!(!Arc::ptr_eq(&a, &text.uri("https://example.com/b")));
        // A short id is stored in the value; a long one is shared like an address.
        assert_eq!(text.id("l7"), *"l7");
        let long = "i".repeat(40);
        assert_eq!(text.id(&long), *long);
        assert!(text.id(&long).is_same(&text.id(&long)));
        assert_eq!(text.ids.strings.len(), 1);
    }

    #[test]
    fn a_table_that_is_full_makes_room_by_dropping_what_nobody_holds() {
        let mut text = LinkText::default();
        // Held by the test, so they must stay.
        let kept: Vec<_> = (0..URIS.entries / 2).map(|n| text.uri(&format!("kept/{n}"))).collect();
        for n in 0..URIS.entries * 4 {
            drop(text.uri(&format!("passing/{n}")));
        }
        assert!(text.uris.strings.len() <= URIS.entries);
        for (n, link) in kept.iter().enumerate() {
            assert!(Arc::ptr_eq(link, &text.uri(&format!("kept/{n}"))), "a held string is never dropped");
        }
        // Past the capacity of what is held, strings are made and not kept.
        let more: Vec<_> = (0..URIS.entries * 2).map(|n| text.uri(&format!("more/{n}"))).collect();
        assert!(text.uris.strings.len() <= URIS.entries);
        assert_eq!(&*more[URIS.entries * 2 - 1], format!("more/{}", URIS.entries * 2 - 1));
    }
}
