//! The text of hyperlinks, shared: one allocation per distinct address and id, however often a
//! program names them.
//!
//! A hyperlink holds its address and its id as `Arc<str>`, so that the cells under it, the pages
//! that hold them and the host that reads them all share one copy. Making those two strings for
//! every OSC 8 was the allocation a link cost, and a program that lists files or prints build
//! errors says the same few hundred addresses again and again. So the terminal keeps the strings it
//! made in two bounded tables keyed by content, and a link whose text is in them costs two lookups
//! and two reference counts.
//!
//! A link without an id of its own gets a fresh one, because nothing ties two such links together:
//! `link-N`, a string of its own. Handing out the ids of links that ended would save that
//! allocation, but then the terminal's state would depend on when a host drops what it read, and
//! the same bytes must always make the same state.
//!
//! Both tables are bounded in entries and bytes, and an entry that cells still hold is never
//! dropped to make room, so a program cannot make the tables grow by naming a million addresses
//! (it gets new strings that are not kept).

use std::collections::HashSet;
use std::fmt::Write as _;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::sync::Arc;

/// The most strings one table keeps, and the most bytes of them. 4,096 addresses cover a big
/// directory listing or a build's error list; the bytes cap is for addresses of kilobytes.
const POOL_ENTRIES: usize = 4096;
const POOL_BYTES: usize = 512 * 1024;

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
        let mut words = bytes.chunks_exact(8);
        for word in &mut words {
            let mut eight = [0u8; 8];
            eight.copy_from_slice(word);
            self.mix(u64::from_le_bytes(eight));
        }
        let rest = words.remainder();
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
    strings: HashSet<Arc<str>, Seeded>,
    bytes: usize,
    /// Strings refused for want of room since the pool last looked for ones nobody holds.
    refused: usize,
}

impl Pool {
    fn new(seed: u64) -> Pool {
        Pool { strings: HashSet::with_hasher(Seeded(seed)), bytes: 0, refused: 0 }
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
        let fits = |pool: &Pool| pool.strings.len() < POOL_ENTRIES && pool.bytes + len <= POOL_BYTES;
        if fits(self) {
            return true;
        }
        // Full: the strings nobody but the pool holds are only a cache and can go. Looking is a
        // pass over the table, so it is done once per eighth of a table of refusals, not for each.
        self.refused += 1;
        if self.refused < POOL_ENTRIES / 8 {
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
        LinkText { uris: Pool::new(seed), ids: Pool::new(seed.rotate_left(32)) }
    }
}

impl LinkText {
    pub(in crate::terminal) fn uri(&mut self, uri: &str) -> Arc<str> {
        self.uris.get(uri)
    }

    pub(in crate::terminal) fn id(&mut self, id: &str) -> Arc<str> {
        self.ids.get(id)
    }

    /// The id of a link that gave none: `link-{number}`.
    pub(in crate::terminal) fn fresh_id(number: u64) -> Arc<str> {
        let mut name = StackText::new();
        let _ = write!(name, "link-{number}");
        Arc::from(name.as_str())
    }
}

/// A short text built without allocating: `write!` into it, then read it back.
struct StackText {
    bytes: [u8; 40],
    len: usize,
}

impl StackText {
    fn new() -> StackText {
        StackText { bytes: [0; 40], len: 0 }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for StackText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len + text.len();
        self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{LinkText, POOL_ENTRIES};

    #[test]
    fn the_same_text_is_one_allocation() {
        let mut text = LinkText::default();
        let (a, b) = (text.uri("https://example.com/a"), text.uri("https://example.com/a"));
        assert!(Arc::ptr_eq(&a, &b));
        assert!(!Arc::ptr_eq(&a, &text.uri("https://example.com/b")));
        // Addresses and ids are separate tables: an id may equal an address without sharing it.
        assert!(!Arc::ptr_eq(&text.id("https://example.com/a"), &a));
        assert_eq!(&*text.id("l7"), "l7");
        assert_eq!(&*LinkText::fresh_id(12), "link-12");
    }

    #[test]
    fn a_table_that_is_full_makes_room_by_dropping_what_nobody_holds() {
        let mut text = LinkText::default();
        // Held by the test, so they must stay.
        let kept: Vec<_> = (0..POOL_ENTRIES / 2).map(|n| text.uri(&format!("kept/{n}"))).collect();
        for n in 0..POOL_ENTRIES * 4 {
            drop(text.uri(&format!("passing/{n}")));
        }
        assert!(text.uris.strings.len() <= POOL_ENTRIES);
        for (n, link) in kept.iter().enumerate() {
            assert!(Arc::ptr_eq(link, &text.uri(&format!("kept/{n}"))), "a held string is never dropped");
        }
        // Past the capacity of what is held, strings are made and not kept.
        let more: Vec<_> = (0..POOL_ENTRIES * 2).map(|n| text.uri(&format!("more/{n}"))).collect();
        assert!(text.uris.strings.len() <= POOL_ENTRIES);
        assert_eq!(&*more[POOL_ENTRIES * 2 - 1], format!("more/{}", POOL_ENTRIES * 2 - 1));
    }
}
