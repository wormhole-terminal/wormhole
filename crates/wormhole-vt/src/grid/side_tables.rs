//! The page's link table, and the hash map shape it shares with the marks table (`graphemes`):
//! what a few cells have beyond their eight bytes.
//!
//! Both are keyed by the cell's offset in the page's cell buffer (`slot * columns + column`), which
//! is stable while rows are moved by relinking, and both are rare enough that a hash map is the
//! right shape: a page of plain text has none, a page of emoji has many marks and no links.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::types::Hyperlink;

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

#[derive(Clone, Debug)]
struct LinkEntry {
    link: Hyperlink,
    /// How many cells are under the link (0 for a free entry).
    refs: u32,
}

/// The cells `start..end` (offsets in the page) are under link `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinkRun {
    start: u32,
    end: u32,
    id: u32,
}

/// The hyperlinks (OSC 8) the page's cells are under, interned: a link over a hundred cells, or
/// over cells of different styles, is one entry that counts its cells.
///
/// Which cells are under which link is kept as runs of offsets, sorted and not overlapping, so a
/// link over a whole line is one record, written with one insertion and found with one binary
/// search (a map entry per cell cost about ten nanoseconds each when the text was written).
/// Cells under one link that sit next to each other in the page share a run, a wrapped line's
/// two rows included; asking for a row clips the runs to it.
#[derive(Clone, Debug, Default)]
pub(super) struct Links {
    entries: Vec<LinkEntry>,
    free: Vec<u32>,
    by_link: HashMap<Hyperlink, u32>,
    /// Which link each flagged cell is under, by start offset.
    runs: Vec<LinkRun>,
    /// Pieces of runs being moved or copied, kept so that doing so allocates nothing.
    scratch: Vec<LinkRun>,
    /// The entry the last interning found: a run of printing under one link asks for it each time.
    last: u32,
    /// The bytes of the ids and addresses the entries hold: a program decides how long they are.
    text: usize,
}

/// The bytes of link ids and addresses a page holds at most. A dense page of real links (`ls
/// --hyperlink` of a large folder) holds a few hundred KiB; past this a new link is not kept, and
/// its text is shown without it.
pub(super) const LINK_TEXT_MAX: usize = 1024 * 1024;

impl Links {
    /// Whether the page can take cells under `link`: it has it already, or room for its text.
    #[inline]
    pub(super) fn has_room_for(&self, link: &Hyperlink) -> bool {
        self.text + link_text(link) <= LINK_TEXT_MAX || self.by_link.contains_key(link)
    }

    /// The id of `link`, counting `n` more cells under it.
    pub(super) fn acquire(&mut self, link: &Hyperlink, n: u32) -> u32 {
        if let Some(entry) = self.entries.get_mut(self.last as usize).filter(|e| e.refs != 0) {
            // Cells printed under one link share its two `Arc`s, so this is two pointer compares.
            if same_link(&entry.link, link) {
                entry.refs += n;
                return self.last;
            }
        }
        if let Some(&id) = self.by_link.get(link) {
            self.entries[id as usize].refs += n;
            self.last = id;
            return id;
        }
        self.text += link_text(link);
        let id = match self.free.pop() {
            Some(id) => {
                self.entries[id as usize] = LinkEntry { link: link.clone(), refs: n };
                id
            }
            None => {
                self.entries.push(LinkEntry { link: link.clone(), refs: n });
                (self.entries.len() - 1) as u32
            }
        };
        self.by_link.insert(link.clone(), id);
        self.last = id;
        id
    }

    pub(super) fn retain(&mut self, id: u32, n: u32) {
        self.entries[id as usize].refs += n;
    }

    pub(super) fn release(&mut self, id: u32, n: u32) {
        let entry = &mut self.entries[id as usize];
        debug_assert!(entry.refs >= n);
        entry.refs = entry.refs.saturating_sub(n);
        if entry.refs == 0 {
            self.text -= link_text(&entry.link);
            self.by_link.remove(&entry.link);
            self.free.push(id);
            if self.last == id {
                self.last = 0;
            }
        }
    }

    /// Put the `n` cells from `offset` under link `id` (the caller has counted them, and they are
    /// not under a link: the cells a write replaces were given back first).
    pub(super) fn attach(&mut self, offset: u32, n: u32, id: u32) {
        if n == 0 {
            return;
        }
        let run = LinkRun { start: offset, end: offset + n, id };
        // Usually the newest text, at the end; otherwise a binary search.
        let at = match self.runs.last() {
            Some(last) if last.start >= offset => self.runs.partition_point(|r| r.start < offset),
            _ => self.runs.len(),
        };
        debug_assert!(at == 0 || self.runs[at - 1].end <= run.start, "cells already under a link");
        debug_assert!(
            self.runs.get(at).is_none_or(|next| next.start >= run.end),
            "cells already under a link"
        );
        // Join the neighbours under the same link, so a line written in pieces stays one record.
        let joins_next = self.runs.get(at).is_some_and(|next| next.start == run.end && next.id == id);
        if at > 0 && self.runs[at - 1].end == run.start && self.runs[at - 1].id == id {
            let end = if joins_next { self.runs.remove(at).end } else { run.end };
            self.runs[at - 1].end = end;
        } else if joins_next {
            self.runs[at].start = run.start;
        } else {
            self.runs.insert(at, run);
        }
    }

    /// Take the cells `a..b` out from under their links, releasing the counts.
    pub(super) fn detach(&mut self, a: u32, b: u32) {
        self.cut(a, b);
        let pieces = std::mem::take(&mut self.scratch);
        for piece in &pieces {
            self.release(piece.id, piece.end - piece.start);
        }
        self.scratch = pieces;
    }

    /// Move the cells `from` (with their links) so that they start at `to`; the counts don't
    /// change. The cells they land on were given back already or are among those moving.
    pub(super) fn relocate(&mut self, from: std::ops::Range<u32>, to: u32) {
        self.cut(from.start, from.end);
        let pieces = std::mem::take(&mut self.scratch);
        for piece in &pieces {
            let at = piece.start - from.start + to;
            self.attach(at, piece.end - piece.start, piece.id);
        }
        self.scratch = pieces;
    }

    /// Put the `n` cells from `to` under the links of the `n` cells from `from` (another row of the
    /// page), counting them; the cells they replace were given back already.
    pub(super) fn copy(&mut self, from: u32, to: u32, n: u32) {
        self.scratch.clear();
        let mut pieces = std::mem::take(&mut self.scratch);
        pieces.extend(self.within(from, from + n).map(|(lo, hi, id)| LinkRun { start: lo, end: hi, id }));
        for piece in &pieces {
            self.retain(piece.id, piece.end - piece.start);
            self.attach(piece.start - from + to, piece.end - piece.start, piece.id);
        }
        self.scratch = pieces;
    }

    /// Remove the cells `a..b` from the runs without releasing anything, leaving the pieces that
    /// were removed (clipped to `a..b`) in `scratch`.
    fn cut(&mut self, a: u32, b: u32) {
        self.scratch.clear();
        let first = self.runs.partition_point(|r| r.end <= a);
        let last = first + self.runs[first..].partition_point(|r| r.start < b);
        if first == last {
            return;
        }
        for run in &self.runs[first..last] {
            self.scratch.push(LinkRun { start: run.start.max(a), end: run.end.min(b), id: run.id });
        }
        let (head, tail) = (self.runs[first], self.runs[last - 1]);
        let left = (head.start < a).then_some(LinkRun { end: a, ..head });
        let right = (tail.end > b).then_some(LinkRun { start: b, ..tail });
        self.runs.splice(first..last, left.into_iter().chain(right));
    }

    /// The runs that touch `a..b`, clipped to it, as `(start, end, id)`.
    pub(super) fn within(&self, a: u32, b: u32) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        let first = self.runs.partition_point(|r| r.end <= a);
        self.runs[first..]
            .iter()
            .take_while(move |r| r.start < b)
            .map(move |r| (r.start.max(a), r.end.min(b), r.id))
    }

    pub(super) fn id_at(&self, offset: u32) -> Option<u32> {
        let at = self.runs.partition_point(|r| r.end <= offset);
        self.runs.get(at).filter(|r| r.start <= offset).map(|r| r.id)
    }

    pub(super) fn link(&self, id: u32) -> &Hyperlink {
        &self.entries[id as usize].link
    }

    /// The link the cell at `offset` is under.
    pub(super) fn at(&self, offset: u32) -> Option<&Hyperlink> {
        self.id_at(offset).map(|id| &self.entries[id as usize].link)
    }

    /// How many cells are under a link.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn len(&self) -> usize {
        self.runs.iter().map(|r| (r.end - r.start) as usize).sum()
    }

    /// Ids and counts of the links in use.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn used(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.entries.iter().enumerate().filter(|(_, e)| e.refs > 0).map(|(id, e)| (id as u32, e.refs))
    }

    #[cfg(any(test, feature = "internals"))]
    pub(super) fn is_indexed(&self, id: u32) -> bool {
        self.entries.get(id as usize).is_some_and(|e| e.refs > 0 && self.by_link.get(&e.link) == Some(&id))
    }

    /// Check that the runs are in order, apart, not empty and under links in use (for the checker).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn verify(&self) -> Result<(), String> {
        let mut end = 0;
        for run in &self.runs {
            if run.start >= run.end || run.start < end {
                return Err(format!(
                    "link run {}..{} is empty or overlaps the one before",
                    run.start, run.end
                ));
            }
            if self.entries.get(run.id as usize).is_none_or(|e| e.refs == 0) {
                return Err(format!(
                    "link run {}..{} is under link {} which is not in use",
                    run.start, run.end, run.id
                ));
            }
            end = run.end;
        }
        Ok(())
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.free.clear();
        self.by_link.clear();
        self.runs.clear();
        self.last = 0;
        self.text = 0;
    }

    pub(super) fn bytes(&self) -> usize {
        self.entries.capacity() * size_of::<LinkEntry>()
            + self.by_link.capacity() * (size_of::<Hyperlink>() + size_of::<u32>() + 1)
            + (self.runs.capacity() + self.scratch.capacity()) * size_of::<LinkRun>()
            + self.text
    }
}

fn link_text(link: &Hyperlink) -> usize {
    link.id.len() + link.uri.len()
}

fn same_link(a: &Hyperlink, b: &Hyperlink) -> bool {
    std::sync::Arc::ptr_eq(&a.id, &b.id) && std::sync::Arc::ptr_eq(&a.uri, &b.uri)
}

#[cfg(test)]
mod tests {
    use super::{LINK_TEXT_MAX, Links};
    use crate::types::Hyperlink;

    fn link(id: &str, uri: &str) -> Hyperlink {
        Hyperlink { id: id.into(), uri: uri.into() }
    }

    #[test]
    fn a_link_is_one_entry_for_all_its_cells() {
        let mut links = Links::default();
        let a = link("1", "https://a");
        let id = links.acquire(&a, 3);
        // An equal link built separately (another `Arc`) is the same entry.
        assert_eq!(links.acquire(&link("1", "https://a"), 2), id);
        assert_ne!(links.acquire(&link("2", "https://a"), 1), id);
        links.attach(10, 1, id);
        assert_eq!(links.at(10), Some(&a));
        assert_eq!(links.at(11), None);
        links.release(id, 5);
        assert!(!links.is_indexed(id));
    }

    /// The runs as `(start, end, id)`.
    fn runs(links: &Links) -> Vec<(u32, u32, u32)> {
        links.within(0, u32::MAX).collect()
    }

    #[test]
    fn detaching_cells_releases_their_share_and_splits_runs() {
        let mut links = Links::default();
        let id = links.acquire(&link("1", "https://a"), 10);
        links.attach(0, 10, id);
        links.detach(3, 5);
        assert_eq!(runs(&links), [(0, 3, id), (5, 10, id)]);
        assert_eq!(links.used().collect::<Vec<_>>(), [(id, 8)]);
        assert_eq!(
            (links.id_at(2), links.id_at(3), links.id_at(4), links.id_at(5)),
            (Some(id), None, None, Some(id))
        );
        // A range over several runs and the gaps between them.
        links.detach(2, 7);
        assert_eq!(runs(&links), [(0, 2, id), (7, 10, id)]);
        links.detach(0, 100);
        assert_eq!(links.used().count(), 0);
        assert_eq!(links.len(), 0);
        // The freed id is reused by the next link.
        assert_eq!(links.acquire(&link("9", "https://z"), 1), id);
        assert!(links.verify().is_ok());
    }

    #[test]
    fn neighbours_under_one_link_share_a_run() {
        let mut links = Links::default();
        let (a, b) = (links.acquire(&link("1", "https://a"), 6), links.acquire(&link("2", "https://b"), 2));
        links.attach(0, 2, a);
        links.attach(4, 2, a);
        links.attach(2, 2, a);
        assert_eq!(runs(&links), [(0, 6, a)]);
        links.attach(6, 2, b);
        links.detach(2, 3);
        links.attach(2, 1, b);
        assert_eq!(runs(&links), [(0, 2, a), (2, 3, b), (3, 6, a), (6, 8, b)]);
        assert_eq!(links.len(), 8);
        assert!(links.verify().is_ok());
    }

    #[test]
    fn cells_move_and_copy_with_their_links() {
        let mut links = Links::default();
        let (a, b) = (links.acquire(&link("1", "https://a"), 5), links.acquire(&link("2", "https://b"), 3));
        links.attach(2, 5, a);
        links.attach(8, 3, b);
        // Shift 2..11 right by 2 (overlapping), as inserting two blank cells does.
        links.relocate(2..11, 4);
        assert_eq!(runs(&links), [(4, 9, a), (10, 13, b)]);
        // And back left, as deleting does.
        links.relocate(4..13, 1);
        assert_eq!(runs(&links), [(1, 6, a), (7, 10, b)]);
        // A copy elsewhere counts the copied cells again.
        links.copy(4, 20, 5);
        assert_eq!(runs(&links), [(1, 6, a), (7, 10, b), (20, 22, a), (23, 25, b)]);
        assert_eq!(links.used().collect::<Vec<_>>(), [(a, 7), (b, 5)]);
        assert!(links.verify().is_ok());
    }

    #[test]
    fn a_page_holds_link_text_up_to_its_cap_and_counts_it_back() {
        let mut links = Links::default();
        let long = |n: usize| link(&n.to_string(), &"x".repeat(64 * 1024));
        let mut ids = Vec::new();
        let mut n = 0;
        while links.has_room_for(&long(n)) {
            ids.push(links.acquire(&long(n), 1));
            n += 1;
        }
        assert!(links.text <= LINK_TEXT_MAX && links.text > LINK_TEXT_MAX - 64 * 1024 - 8);
        // A link the page has already is always room; one given back makes room again.
        assert!(links.has_room_for(&long(0)));
        links.release(ids[1], 1);
        assert!(links.has_room_for(&long(n)));
        links.clear();
        assert_eq!(links.text, 0);
    }
}
