//! Where the checkpoints go, and how a payload is written with them.
//!
//! The payload itself is never changed: it stays the same bytes for every terminal and every run.
//! A plan is a list of offsets into it; at each one the writer sends the query `ESC [ 5 n` between
//! that byte and the next, inside the write that carries them, as one `writev` call
//! (`data[..k]`, query, `data[k..]`). The writes have the size and number they have without the
//! check; only the four bytes of a query are added to one.

use std::io::{self, IoSlice, Write};

use super::scan::Scanner;
use crate::bench::workloads::{Payload, Writes};
use crate::canvas::Rng;

/// DSR 5, "are you all right?". A terminal answers `ESC [ 0 n` (see `Input::count_status_replies`).
pub const QUERY: &[u8] = b"\x1b[5n";

/// The average distance between checkpoints.
pub const DEFAULT_EVERY: usize = 256 * 1024;
/// The distance is cut to the payload's length over this, so a small payload is checked about this
/// often too (`--size 1` gets some 30 checkpoints, not 4).
pub const SPREAD: usize = 32;
/// The most buffers handed to one `writev` (the system's limit is 1,024).
const MAX_IOV: usize = 512;

/// Whether a query may go between two bytes of the payload. Not inside a sequence or a character
/// (`scan`), and only between two ASCII bytes with a printable one, CR or LF before: a terminal that
/// clusters graphemes (mode 2027) must not see a query between the parts of one (the letters of a
/// flag, a character and its variation selector or skin tone, a joiner and what it joins), and
/// nothing outside ASCII has to be judged.
fn may_split(before: u8, after: u8, ground: bool) -> bool {
    ground && (matches!(before, 0x20..=0x7e) || before == b'\r' || before == b'\n') && after < 0x80
}

/// The checkpoints of one run: the number of payload bytes sent before each query, ascending.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub cuts: Vec<usize>,
    /// What the gaps were drawn from: the same seed, payload and distance give the same checkpoints.
    /// `None` for a run without any.
    pub seed: Option<u64>,
}

impl Plan {
    /// No checkpoints: the payload goes out as it always did.
    pub fn none() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.cuts.len()
    }

    /// Checkpoints about `every` bytes apart, the gaps drawn from `seed` between half and one and a
    /// half times that; each goes at the first byte past its place where `may_split` allows it.
    /// Where the payload has few such places there are few checkpoints, and the run says so.
    ///
    /// Why the gaps vary and the query sits anywhere in a write: a terminal that reads in ticks and
    /// keeps only the end of each, or the end of each read, would otherwise see every query at the
    /// same place in its tick, or the end of its read, always in the part it keeps.
    /// There is no checkpoint at the very end: it comes right before the cursor report, in the bytes a
    /// terminal that drops input keeps, and proves nothing.
    pub fn new(payload: &Payload, every: usize, seed: u64) -> Self {
        let bytes = &payload.bytes;
        let every = every.min(bytes.len() / SPREAD).max(1);
        let mut rng = Rng::new(seed);
        let mut gap = || (every / 2 + rng.below(every)).max(1);
        let mut cuts = Vec::new();
        let mut next = gap();
        let mut scanner = Scanner::new();
        for (i, pair) in bytes.windows(2).enumerate() {
            scanner.feed(pair[0]);
            let at = i + 1;
            if at >= next && may_split(pair[0], pair[1], scanner.is_ground()) {
                cuts.push(at);
                next = at + gap();
            }
        }
        Self { cuts, seed: Some(seed) }
    }
}

/// One write of a payload: the data and the queries inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub data: &'a [u8],
    /// Where `data` starts in the payload.
    base: usize,
    /// The plan's offsets that fall in this write (absolute: `base < at <= base + data.len()`).
    cuts: &'a [usize],
}

impl Chunk<'_> {
    /// How many queries the write carries.
    pub fn queries(&self) -> usize {
        self.cuts.len()
    }

    #[cfg(test)]
    pub fn plain(data: &[u8]) -> Chunk<'_> {
        Chunk { data, base: 0, cuts: &[] }
    }

    #[cfg(test)]
    pub fn with_cuts<'a>(data: &'a [u8], cuts: &'a [usize]) -> Chunk<'a> {
        Chunk { data, base: 0, cuts }
    }
}

/// The writes of `payload` with the plan's queries in them: the same writes as without the plan, each
/// with four more bytes for every query inside it.
pub fn chunks<'a>(payload: &'a Payload, plan: &'a Plan) -> Chunks<'a> {
    Chunks { writes: payload.writes(), cuts: &plan.cuts, used: 0, pos: 0 }
}

pub struct Chunks<'a> {
    writes: Writes<'a>,
    cuts: &'a [usize],
    /// How many of the cuts went out already.
    used: usize,
    /// Payload bytes handed out so far.
    pos: usize,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Chunk<'a>;

    fn next(&mut self) -> Option<Chunk<'a>> {
        let data = self.writes.next()?;
        let end = self.pos + data.len();
        let carried = self.cuts[self.used..].iter().take_while(|&&at| at <= end).count();
        let cuts = &self.cuts[self.used..self.used + carried];
        debug_assert!(cuts.iter().all(|&at| at > self.pos), "a cut that no write can carry");
        let chunk = Chunk { data, base: self.pos, cuts };
        self.used += carried;
        self.pos = end;
        Some(chunk)
    }
}

/// Writes a chunk in one call to the kernel, queries and all (`writev`), so a checkpoint costs no write
/// of its own and the write keeps its place in the stream.
pub fn write_chunk(out: &mut impl Write, chunk: Chunk<'_>) -> io::Result<()> {
    if chunk.cuts.is_empty() {
        return out.write_all(chunk.data);
    }
    // Only non-empty buffers: an empty one at the front would look like a write of nothing.
    let mut bufs = Vec::with_capacity(2 * chunk.cuts.len() + 1);
    let mut from = 0;
    for &at in chunk.cuts {
        let k = at - chunk.base;
        if k > from {
            bufs.push(IoSlice::new(&chunk.data[from..k]));
        }
        bufs.push(IoSlice::new(QUERY));
        from = k;
    }
    if from < chunk.data.len() {
        bufs.push(IoSlice::new(&chunk.data[from..]));
    }
    let mut bufs: &mut [IoSlice<'_>] = &mut bufs;
    while !bufs.is_empty() {
        let batch = bufs.len().min(MAX_IOV);
        match out.write_vectored(&bufs[..batch]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            // Drops the buffers that went out whole and trims the one that went out in part.
            Ok(n) => IoSlice::advance_slices(&mut bufs, n),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::workloads;
    use crate::term::{Size, parse_replies};

    fn text(len: usize) -> Payload {
        Payload {
            bytes: b"0123456789abcdef\r\n".iter().copied().cycle().take(len).collect(),
            ..Payload::default()
        }
    }

    /// The payload and the queries as the chunks send them: the bytes of the stream, and for each
    /// query the payload offset it sits at.
    fn replayed(p: &Payload, plan: &Plan) -> (Vec<u8>, Vec<usize>) {
        let mut out = Recorder::default();
        for s in chunks(p, plan) {
            write_chunk(&mut out, s).expect("write");
        }
        let mut data = Vec::new();
        let mut cuts = Vec::new();
        let mut i = 0;
        while i < out.bytes.len() {
            if out.bytes[i..].starts_with(QUERY) && !p.bytes[data.len()..].starts_with(QUERY) {
                cuts.push(data.len());
                i += QUERY.len();
            } else {
                data.push(out.bytes[i]);
                i += 1;
            }
        }
        (data, cuts)
    }

    /// A writer like a file: every call is one call to the kernel, and a vectored one takes all.
    #[derive(Default)]
    struct Recorder {
        bytes: Vec<u8>,
        calls: usize,
    }

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            self.calls += 1;
            let before = self.bytes.len();
            for b in bufs {
                self.bytes.extend_from_slice(b);
            }
            Ok(self.bytes.len() - before)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn checkpoints_are_spread_by_the_distance_asked() {
        let p = text(4_000_000);
        let plan = Plan::new(&p, 100_000, 7);
        assert!((30..=50).contains(&plan.len()), "{}", plan.len());
        assert!(plan.cuts.windows(2).all(|w| w[0] < w[1]));
        // Gaps from half the distance to one and a half times it, and on to the next place that allows
        // one (every 18 bytes here).
        let gaps: Vec<usize> = plan.cuts.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|g| (50_000..=150_020).contains(g)), "{gaps:?}");
        // Another seed puts them elsewhere, the same seed in the same places.
        assert_ne!(plan, Plan::new(&p, 100_000, 8));
        assert_eq!(plan, Plan::new(&p, 100_000, 7));
        assert_eq!(plan.seed, Some(7));
        // Never one at the very end, where a terminal that drops input still answers.
        assert!(plan.cuts.iter().all(|&c| c < p.bytes.len()));
    }

    #[test]
    fn a_small_payload_is_checked_about_as_often_and_an_empty_one_not_at_all() {
        let plan = Plan::new(&text(100_000), DEFAULT_EVERY, 1);
        assert!((25..=50).contains(&plan.len()), "{}", plan.len());
        assert!(Plan::new(&Payload::default(), DEFAULT_EVERY, 1).cuts.is_empty());
    }

    #[test]
    fn the_data_is_the_payload_and_the_writes_are_the_same_whatever_the_plan() {
        for seed in 0..5 {
            let p = text(1_000_003);
            let plan = Plan::new(&p, 100_000, seed);
            let (data, cuts) = replayed(&p, &plan);
            assert!(data == p.bytes, "seed {seed}");
            assert_eq!(cuts, plan.cuts);
            // Same number of writes, each with its own bytes; the queries are the four extra.
            let plain: Vec<usize> = p.writes().map(|w| w.len()).collect();
            let with: Vec<usize> = chunks(&p, &plan).map(|s| s.data.len()).collect();
            assert_eq!(plain, with);
            assert_eq!(chunks(&p, &plan).map(|s| s.queries()).sum::<usize>(), plan.len());
        }
        let p = text(workloads::CHUNK * 3 + 17);
        let none = Plan::none();
        assert!(chunks(&p, &none).map(|s| s.data).eq(p.writes()));
    }

    #[test]
    fn a_write_with_queries_in_it_is_one_call_and_the_others_are_one_call_each() {
        let p = text(workloads::CHUNK * 20);
        let plan = Plan::new(&p, 20_000, 3);
        assert!(plan.len() > 20, "{}", plan.len());
        let mut out = Recorder::default();
        for s in chunks(&p, &plan) {
            write_chunk(&mut out, s).expect("write");
        }
        // Many writes carry two or three queries; none takes more than its one call.
        assert_eq!(out.calls, p.write_count());
        assert_eq!(out.bytes.len(), p.bytes.len() + plan.len() * QUERY.len());
    }

    #[test]
    fn chunked_payloads_keep_their_writes_and_may_be_cut_inside_one() {
        let bytes = "ab red text, then more text 123\r\n".repeat(3000).into_bytes();
        let mut sizes = Vec::new();
        let mut left = bytes.len();
        let mut n = 2;
        while left > 0 {
            let s = n.min(left);
            sizes.push(s);
            left -= s;
            n = n % 15 + 2;
        }
        let p = Payload { bytes, frames: 0, chunks: Some(sizes.clone()) };
        let plan = Plan::new(&p, 4000, 3);
        assert!(plan.len() >= 16, "{}", plan.len());
        assert_eq!(chunks(&p, &plan).map(|s| s.data.len()).collect::<Vec<_>>(), sizes);
        assert_eq!(replayed(&p, &plan).0, p.bytes);
        // Some query sits inside a token: the rule is about bytes, not about where writes end.
        let ends: Vec<usize> = sizes
            .iter()
            .scan(0, |s, n| {
                *s += n;
                Some(*s)
            })
            .collect();
        assert!(plan.cuts.iter().any(|c| !ends.contains(c)));
    }

    #[test]
    fn a_checkpoint_is_never_inside_a_sequence_a_character_or_a_grapheme() {
        // The oracles are not the scanner the plan used: the character is checked by std, the
        // sequences by the parser that reads what terminals send (it leaves an unfinished one unused),
        // and the grapheme by looking at the neighbours.
        let size = Size { cols: 100, rows: 30, px_w: 0, px_h: 0 };
        for name in workloads::NAMES.iter().filter(|n| **n != "roundtrip") {
            let Some(p) = workloads::payload(name, size, 1_500_000) else { panic!("no payload {name}") };
            let whole_is_text = std::str::from_utf8(&p.bytes).is_ok();
            // No C1 control in any payload (the scanner does not model them), raw or as UTF-8.
            assert!(!p.bytes.windows(2).any(|w| w[0] == 0xc2 && (0x80..0xa0).contains(&w[1])), "{name}");
            assert!(!p.bytes.iter().any(|b| (0x80..0xa0).contains(b)) || whole_is_text, "{name}");
            for seed in 0..3 {
                let plan = Plan::new(&p, 9_000, seed);
                // Each stretch from one checkpoint to the next is checked from a place already known to
                // be closed, which keeps the oracle linear.
                let mut prev = 0;
                for &at in &plan.cuts {
                    let (before, after) = (p.bytes[at - 1], p.bytes[at]);
                    assert!(
                        (0x20..0x7f).contains(&before) || before == b'\r' || before == b'\n',
                        "{name}: cut {at} after {before:#x}"
                    );
                    assert!(after < 0x80, "{name}: cut {at} before {after:#x}");
                    let stretch = &p.bytes[prev..at];
                    if whole_is_text {
                        assert!(std::str::from_utf8(stretch).is_ok(), "{name}: cut {at} inside a character");
                    }
                    let (_, used) = parse_replies(stretch);
                    assert_eq!(used, stretch.len(), "{name}: cut {at} inside a sequence");
                    prev = at;
                }
                let (data, _) = replayed(&p, &plan);
                assert!(data == p.bytes, "{name}");
            }
        }
    }

    #[test]
    fn no_checkpoint_splits_a_grapheme_of_glyphstorm() {
        // glyphstorm is the payload of emoji sequences, flags, skin tones and combining marks.
        let size = Size { cols: 100, rows: 30, px_w: 0, px_h: 0 };
        let p = workloads::payload("glyphstorm", size, 4_000_000).expect("payload");
        let ext = |c: char| {
            c == '\u{200d}'
                || c == '\u{fe0f}'
                || ('\u{1f3fb}'..='\u{1f3ff}').contains(&c)
                || ('\u{300}'..='\u{36f}').contains(&c)
        };
        let ri = |c: char| ('\u{1f1e6}'..='\u{1f1ff}').contains(&c);
        let text = std::str::from_utf8(&p.bytes).expect("glyphstorm is text");
        let mut total = 0;
        for seed in 0..20 {
            for &at in &Plan::new(&p, DEFAULT_EVERY, seed).cuts {
                let prev = text[..at].chars().next_back();
                let next = text[at..].chars().next();
                let joins = match (prev, next) {
                    (_, Some(n)) if ext(n) => true,
                    (Some('\u{200d}'), _) => true,
                    (Some(a), Some(b)) => ri(a) && ri(b),
                    _ => false,
                };
                assert!(!joins, "cut {at} between {prev:?} and {next:?}");
                total += 1;
            }
        }
        assert!(total > 0);
    }

    /// The checks that count answers and tell a query of ours from the payload's own rely on the payload
    /// never asking for a status or a cursor report itself.
    #[test]
    fn no_payload_asks_the_terminal_a_question_of_its_own() {
        let size = Size { cols: 120, rows: 40, px_w: 0, px_h: 0 };
        for name in workloads::NAMES.iter().filter(|n| **n != "roundtrip") {
            for target in [50_000, 1_000_000, 4_000_000] {
                let Some(p) = workloads::payload(name, size, target) else { panic!("no payload {name}") };
                for query in [QUERY, b"\x1b[6n"] {
                    assert!(!p.bytes.windows(query.len()).any(|w| w == query), "{name} at {target} bytes");
                }
            }
        }
    }

    #[test]
    fn every_payload_gets_enough_checkpoints_at_the_default_size_and_a_small_one() {
        let size = Size { cols: 120, rows: 40, px_w: 0, px_h: 0 };
        for name in workloads::NAMES.iter().filter(|n| **n != "roundtrip") {
            for target in [1_000_000, 4_000_000] {
                let Some(p) = workloads::payload(name, size, target) else { panic!("no payload {name}") };
                let n = Plan::new(&p, DEFAULT_EVERY, 5).len();
                assert!(n as u64 >= super::super::ENOUGH, "{name} at {target} bytes: {n}");
            }
        }
    }

    #[test]
    fn a_vectored_write_that_goes_out_in_pieces_arrives_whole_and_in_order() {
        struct Dribble(Vec<u8>);
        impl Write for Dribble {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let n = buf.len().min(3);
                self.0.extend_from_slice(&buf[..n]);
                Ok(n)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut out = Dribble(Vec::new());
        write_chunk(&mut out, Chunk::with_cuts(b"hello world", &[5, 11])).expect("write");
        assert_eq!(out.0, b"hello\x1b[5n world\x1b[5n");
        let mut out = Dribble(Vec::new());
        write_chunk(&mut out, Chunk::plain(b"plain")).expect("write");
        assert_eq!(out.0, b"plain");
    }

    /// Prints how many checkpoints each payload gets (`cargo test measure_checkpoint_counts -- --ignored
    /// --nocapture`): the figures in the README come from here.
    #[test]
    #[ignore = "a measurement, not a check"]
    fn measure_checkpoint_counts() {
        let size = Size { cols: 120, rows: 40, px_w: 0, px_h: 0 };
        for target in [50_000, 1_000_000, 16_000_000] {
            for name in workloads::NAMES.iter().filter(|n| **n != "roundtrip") {
                let Some(p) = workloads::payload(name, size, target) else { continue };
                let counts: Vec<usize> =
                    (0..30).map(|seed| Plan::new(&p, DEFAULT_EVERY, seed).len()).collect();
                let mean = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
                let (min, max) = (counts.iter().min().unwrap_or(&0), counts.iter().max().unwrap_or(&0));
                println!(
                    "MEASURE {target:>9} {name:<12} bytes {:>9} checkpoints min {min:>3} mean {mean:>6.1} max {max:>3}  one per {:>7.0} bytes",
                    p.bytes.len(),
                    p.bytes.len() as f64 / mean.max(1e-9)
                );
            }
        }
    }
}
