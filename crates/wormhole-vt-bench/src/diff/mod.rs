//! Differential testing: the same bytes to two engines, then compare what they show.
//!
//! [`compare`] feeds one stream to two adapters (each in the pieces it is told to) and compares
//! the screen, the cursor, the replies and the rest; [`random::StreamGen`] makes streams that
//! lean toward valid sequences; [`fuzz`] runs many of them and [`minimize`] cuts a failing
//! stream down to what still makes the engines differ. Here it runs the engine against itself (the
//! harness and the adapter agree with themselves, and the engine gives the same screen however
//! the bytes are cut); `tools/vt-compare` runs `fuzz::<AlacrittyEngine, WormholeEngine>` as the
//! parity check against another engine, and the [`Compare`] switches narrow it to what the other
//! engine does.

pub mod random;

use std::fmt::Write as _;

use crate::engine::Engine;
use crate::rng::Rng;
use crate::vtt::show_bytes;

/// How a stream is cut into `feed` calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Whole,
    /// Pieces of this many bytes.
    Bytes(usize),
    /// Pieces of 1 to `max` bytes, the lengths drawn from this seed.
    Random {
        seed: u64,
        max: usize,
    },
}

fn feed<E: Engine>(engine: &mut E, bytes: &[u8], split: Split) {
    match split {
        Split::Whole => engine.feed(bytes),
        Split::Bytes(n) => bytes.chunks(n.max(1)).for_each(|piece| engine.feed(piece)),
        Split::Random { seed, max } => {
            let mut rng = Rng::new(seed);
            let mut rest = bytes;
            while !rest.is_empty() {
                let take = rng.range(1, max.max(1)).min(rest.len());
                let (piece, tail) = rest.split_at(take);
                engine.feed(piece);
                rest = tail;
            }
        }
    }
}

/// What to compare. Everything by default; turn off what an engine cannot do yet.
#[derive(Debug, Clone, Copy)]
pub struct Compare {
    /// The text of every row and whether it is wrapped.
    pub text: bool,
    /// Every cell's width, colors, attributes and link.
    pub cells: bool,
    pub cursor: bool,
    pub replies: bool,
    pub title: bool,
    pub alt_screen: bool,
    pub scrollback: bool,
}

impl Default for Compare {
    fn default() -> Self {
        Compare {
            text: true,
            cells: true,
            cursor: true,
            replies: true,
            title: true,
            alt_screen: true,
            scrollback: true,
        }
    }
}

/// The screen and everything beside it the engines are asked about.
#[derive(Debug, Clone, Copy)]
pub struct Setup {
    pub cols: usize,
    pub rows: usize,
    pub scrollback: usize,
    pub compare: Compare,
}

impl Setup {
    pub fn new(cols: usize, rows: usize) -> Setup {
        Setup { cols, rows, scrollback: 50, compare: Compare::default() }
    }
}

/// Where two engines differ.
#[derive(Debug)]
pub struct Mismatch {
    pub engines: (&'static str, &'static str),
    /// What differs, such as "row 3 text" or "cell 2,4 attrs".
    pub what: String,
    pub a: String,
    pub b: String,
    /// Each engine's screen, as text.
    pub screens: (String, String),
    pub stream: Vec<u8>,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} and {} differ: {}", self.engines.0, self.engines.1, self.what)?;
        writeln!(f, "  {}: {}", self.engines.0, self.a)?;
        writeln!(f, "  {}: {}", self.engines.1, self.b)?;
        writeln!(f, "stream ({} bytes): {}", self.stream.len(), show_bytes(&self.stream))?;
        writeln!(f, "screen of {}:\n{}", self.engines.0, self.screens.0)?;
        write!(f, "screen of {}:\n{}", self.engines.1, self.screens.1)
    }
}

fn screen_text<E: Engine>(engine: &E) -> String {
    let (_, rows) = engine.size();
    let mut out = String::new();
    for row in 0..rows {
        let _ = writeln!(out, "  {:>2} |{}|", row + 1, engine.row_text(row));
    }
    let cursor = engine.cursor();
    let _ = write!(
        out,
        "  cursor {},{}{}",
        cursor.row + 1,
        cursor.col + 1,
        if cursor.visible { "" } else { " hidden" }
    );
    out
}

/// Feed `stream` to an `A` cut as `split_a` says and a `B` cut as `split_b`, and compare. The
/// first difference found is returned.
pub fn compare<A: Engine, B: Engine>(
    stream: &[u8],
    setup: &Setup,
    split_a: Split,
    split_b: Split,
) -> Result<(), Box<Mismatch>> {
    let mut a = A::new(setup.cols, setup.rows, setup.scrollback);
    let mut b = B::new(setup.cols, setup.rows, setup.scrollback);
    feed(&mut a, stream, split_a);
    feed(&mut b, stream, split_b);
    match first_difference(&mut a, &mut b, &setup.compare) {
        None => Ok(()),
        Some((what, left, right)) => Err(Box::new(Mismatch {
            engines: (A::NAME, B::NAME),
            what,
            a: left,
            b: right,
            screens: (screen_text(&a), screen_text(&b)),
            stream: stream.to_vec(),
        })),
    }
}

fn first_difference<A: Engine, B: Engine>(
    a: &mut A,
    b: &mut B,
    what: &Compare,
) -> Option<(String, String, String)> {
    let diff = |name: String, x: String, y: String| (x != y).then_some((name, x, y));
    if a.size() != b.size() {
        return Some(("size".into(), format!("{:?}", a.size()), format!("{:?}", b.size())));
    }
    let (cols, rows) = a.size();
    if what.replies
        && let Some(d) = diff("replies".into(), show_bytes(&a.take_replies()), show_bytes(&b.take_replies()))
    {
        return Some(d);
    }
    if what.title
        && let Some(d) = diff("title".into(), format!("{:?}", a.title()), format!("{:?}", b.title()))
    {
        return Some(d);
    }
    if what.alt_screen
        && let Some(d) =
            diff("alternate screen".into(), a.alt_screen().to_string(), b.alt_screen().to_string())
    {
        return Some(d);
    }
    if what.cursor {
        let (x, y) = (a.cursor(), b.cursor());
        let show = |c: crate::Cursor| {
            format!("row {} col {}{}", c.row + 1, c.col + 1, if c.visible { "" } else { " hidden" })
        };
        if let Some(d) = diff("cursor".into(), show(x), show(y)) {
            return Some(d);
        }
    }
    for row in 0..rows {
        if what.text {
            if let Some(d) = diff(
                format!("row {} text", row + 1),
                format!("{:?}", a.row_text(row)),
                format!("{:?}", b.row_text(row)),
            ) {
                return Some(d);
            }
            if let Some(d) = diff(
                format!("row {} wrapped", row + 1),
                a.row_wrapped(row).to_string(),
                b.row_wrapped(row).to_string(),
            ) {
                return Some(d);
            }
        }
        if what.cells {
            for col in 0..cols {
                let (x, y) = (a.cell(row, col), b.cell(row, col));
                if x != y {
                    return Some((
                        format!("cell {},{}", row + 1, col + 1),
                        format!("{x:?}"),
                        format!("{y:?}"),
                    ));
                }
            }
        }
    }
    if what.scrollback {
        if let Some(d) =
            diff("history length".into(), a.scrollback_len().to_string(), b.scrollback_len().to_string())
        {
            return Some(d);
        }
        for i in 0..a.scrollback_len() {
            if let Some(d) = diff(
                format!("history line {}", i + 1),
                format!("{:?}", a.scrollback_text(i)),
                format!("{:?}", b.scrollback_text(i)),
            ) {
                return Some(d);
            }
        }
    }
    None
}

/// Cut `stream` down to a shorter one the two engines still differ on: remove pieces (halves,
/// then quarters, down to single bytes) for as long as the difference stays.
pub fn minimize<A: Engine, B: Engine>(
    stream: &[u8],
    setup: &Setup,
    split_a: Split,
    split_b: Split,
) -> Vec<u8> {
    let differs = |s: &[u8]| compare::<A, B>(s, setup, split_a, split_b).is_err();
    let mut current = stream.to_vec();
    if !differs(&current) {
        return current;
    }
    let mut size = current.len().div_ceil(2).max(1);
    loop {
        let mut start = 0;
        while start < current.len() {
            let end = (start + size).min(current.len());
            let mut candidate = current[..start].to_vec();
            candidate.extend_from_slice(&current[end..]);
            if !candidate.is_empty() && differs(&candidate) {
                current = candidate;
            } else {
                start += size;
            }
        }
        if size == 1 {
            return current;
        }
        size = size.div_ceil(2);
    }
}

/// How many streams to try, how long they are and where they start.
#[derive(Debug, Clone, Copy)]
pub struct Fuzz {
    pub streams: usize,
    pub length: usize,
    pub seed: u64,
    pub split_a: Split,
    pub split_b: Split,
    /// Streams of ASCII only (see [`random::StreamGen::ascii_only`]).
    pub ascii_only: bool,
}

/// Compare `A` and `B` on `fuzz.streams` random streams. The first difference is returned with
/// the stream cut down by [`minimize`] (the full mismatch is for the shortened stream).
pub fn fuzz<A: Engine, B: Engine>(fuzz: &Fuzz, setup: &Setup) -> Result<(), Box<Mismatch>> {
    let mut generator = random::StreamGen::new(fuzz.seed, setup.cols, setup.rows);
    if fuzz.ascii_only {
        generator = generator.ascii_only();
    }
    for _ in 0..fuzz.streams {
        let stream = generator.stream(fuzz.length);
        if compare::<A, B>(&stream, setup, fuzz.split_a, fuzz.split_b).is_err() {
            let short = minimize::<A, B>(&stream, setup, fuzz.split_a, fuzz.split_b);
            return compare::<A, B>(&short, setup, fuzz.split_a, fuzz.split_b);
        }
    }
    Ok(())
}
