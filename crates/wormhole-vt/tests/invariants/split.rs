//! Split invariance: a stream fed whole, cut at any one byte, or cut into random pieces leaves the
//! terminal in the same state.

use wormhole_vt::{Event, StateDump, Terminal};

use crate::generator::*;
use crate::rng::Rng;

/// Everything a host can see of a terminal after some input.
struct Observed {
    state: StateDump,
    replies: Vec<u8>,
    events: Vec<Event>,
}

fn observe(mut term: Terminal) -> Observed {
    let state = term.dump_state();
    let replies = term.take_replies();
    let events = term.events().collect();
    Observed { state, replies, events }
}

fn assert_same(seed: u64, bytes: &[u8], pieces: &str, whole: &Observed, split: &Observed) {
    let state = if whole.state == split.state {
        String::new()
    } else {
        format!("state differs:\n{}", whole.state.diff(&split.state, 4))
    };
    let replies = if whole.replies == split.replies {
        String::new()
    } else {
        format!("replies differ:\n  whole: {}\n  split: {}\n", show(&whole.replies), show(&split.replies))
    };
    let events = if whole.events == split.events {
        String::new()
    } else {
        format!("events differ:\n  whole: {:?}\n  split: {:?}\n", whole.events, split.events)
    };
    assert!(
        state.is_empty() && replies.is_empty() && events.is_empty(),
        "seed {seed}, fed {pieces}\n{state}{replies}{events}stream ({} bytes): {}",
        bytes.len(),
        show(&bytes[..bytes.len().min(700)])
    );
}

/// Seeds `from..from + count` of a size, fed whole and cut at every byte.
fn split_at_every_byte(from: u64, count: u64, len: usize, mix: Mix) {
    for seed in from..from + count {
        let (cols, rows) = (8 + (seed as usize % 3) * 9, 4 + (seed as usize % 4) * 3);
        let bytes = stream(seed, len, cols, rows, mix);
        let mut whole = terminal(cols, rows);
        whole.feed(&bytes);
        let whole = observe(whole);
        for cut in 1..bytes.len() {
            let mut split = terminal(cols, rows);
            split.feed(&bytes[..cut]);
            split.feed(&bytes[cut..]);
            assert_same(seed, &bytes, &format!("in two at byte {cut}"), &whole, &observe(split));
        }
    }
}

/// Seeds fed whole and in random pieces, one byte at a time among them.
fn split_in_random_pieces(from: u64, count: u64, len: usize, mix: Mix) {
    for seed in from..from + count {
        let (cols, rows) = (10 + (seed as usize % 5) * 13, 5 + (seed as usize % 3) * 8);
        let bytes = stream(seed, len, cols, rows, mix);
        let mut whole = terminal(cols, rows);
        whole.feed(&bytes);
        let whole = observe(whole);
        for round in 0..4u64 {
            let mut rng = Rng::new(seed * 31 + round);
            let max = [1, 3, 17, 300][round as usize];
            let mut split = terminal(cols, rows);
            let (mut at, mut sizes) = (0, Vec::new());
            while at < bytes.len() {
                let n = rng.range(1, max).min(bytes.len() - at);
                split.feed(&bytes[at..at + n]);
                if sizes.len() < 24 {
                    sizes.push(n);
                }
                at += n;
            }
            assert_same(
                seed,
                &bytes,
                &format!("in pieces of 1..={max} (first sizes {sizes:?})"),
                &whole,
                &observe(split),
            );
        }
    }
}

#[test]
fn feeding_in_two_at_any_byte_is_feeding_whole() {
    split_at_every_byte(1, 60, 400, TEXTY);
}

#[test]
fn feeding_in_two_at_any_byte_is_feeding_whole_with_garbage() {
    split_at_every_byte(1_000, 60, 400, HOSTILE);
}

#[test]
fn feeding_in_random_pieces_is_feeding_whole() {
    split_in_random_pieces(2_000, 120, 6_000, TEXTY);
}

#[test]
fn feeding_in_random_pieces_is_feeding_whole_with_garbage() {
    split_in_random_pieces(3_000, 120, 6_000, HOSTILE);
}

/// A wide cluster, a combining mark or a link cut in the middle is the case worth the most; this
/// one cuts only there.
#[test]
fn feeding_a_stream_of_clusters_and_links_one_byte_at_a_time_is_feeding_whole() {
    for seed in 0..30u64 {
        let mut rng = Rng::new(seed);
        let mut bytes = b"\x1b[?2027h".to_vec();
        for _ in 0..40 {
            match rng.below(4) {
                0 => bytes.extend_from_slice(rng.pick(UNICODE).as_bytes()),
                1 => bytes
                    .extend_from_slice(format!("\x1b]8;;https://e.com/{}\x1b\\", rng.below(9)).as_bytes()),
                2 => bytes.extend_from_slice(b"\x1b]8;;\x07"),
                _ => bytes.extend_from_slice(rng.pick(WORDS).as_bytes()),
            }
        }
        let mut whole = terminal(11, 5);
        whole.feed(&bytes);
        let whole = observe(whole);
        let mut split = terminal(11, 5);
        for b in &bytes {
            split.feed(std::slice::from_ref(b));
        }
        assert_same(seed, &bytes, "byte by byte", &whole, &observe(split));
    }
}

#[test]
#[ignore = "ten times the seeds and longer streams: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn split_invariance_many_seeds() {
    split_at_every_byte(10_000, 80, 600, TEXTY);
    split_at_every_byte(20_000, 80, 600, HOSTILE);
    split_in_random_pieces(30_000, 250, 20_000, HOSTILE);
}
