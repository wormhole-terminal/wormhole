//! The hot frames cost no allocation in steady state: `Out` and `Input` encode into a reused
//! buffer and decode out of the decoder's own, however the reads cut the stream, and so does
//! every other message on the way out. A counting allocator watches this thread.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.
#![allow(unsafe_code)] // only the counting allocator below, which forwards to the system's

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use common::*;
use wormhole_wire::*;

thread_local! {
    /// Allocations this thread has made. A `const` cell without a destructor: reading it allocates nothing.
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

fn count() {
    let _ = COUNT.try_with(|c| c.set(c.get() + 1));
}

// SAFETY: every call is forwarded unchanged to `System`, which upholds `GlobalAlloc`'s contract;
// the counting touches no memory of the caller.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations() -> usize {
    COUNT.with(Cell::get)
}

/// A flood's frames: output and input of every size from one byte to a full chunk.
fn flood(data: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut offset = 0u64;
    for (i, len) in [1usize, 7, 100, 1000, 4096, 8000, MAX_DATA, 300, MAX_DATA, 2]
        .into_iter()
        .cycle()
        .take(400)
        .enumerate()
    {
        let bytes = &data[i % 5..i % 5 + len.min(data.len() - 5)];
        if i % 4 == 3 {
            encode(&mut stream, 1, flags::URGENT, &Message::Input(bytes)).expect("encodes");
        } else {
            encode(&mut stream, 1, 0, &Message::Out(Out { offset, bytes })).expect("encodes");
            offset += bytes.len() as u64;
        }
    }
    encode(&mut stream, 1, 0, &Message::Ack(offset)).expect("encodes");
    stream
}

/// One pass over `stream` with reads cut by a fixed sequence; returns the frames and bytes seen.
fn decode_pass(dec: &mut Decoder, stream: &[u8], seed: u64) -> (usize, usize) {
    let mut rng = Rng::new(seed);
    let mut src = Chunked { data: stream, step: move || 1 + rng.usize(100_000) };
    let (mut frames, mut bytes) = (0, 0);
    loop {
        while let Some(frame) = dec.next_frame().expect("valid") {
            frames += 1;
            if let Message::Out(Out { bytes: b, .. }) | Message::Input(b) = frame.message {
                bytes += b.len();
            }
        }
        if dec.fill(&mut src).expect("reads") == 0 {
            return (frames, bytes);
        }
    }
}

#[test]
fn the_counter_counts() {
    let before = allocations();
    let v: Vec<u8> = Vec::with_capacity(4096);
    assert!(allocations() > before, "a counting test that cannot see allocations proves nothing");
    drop(v);
}

#[test]
fn hot_frames_decode_without_allocating() {
    let mut rng = Rng::new(1);
    let data = rng.bytes(MAX_DATA + 5);
    let stream = flood(&data);
    let mut dec = Decoder::new();

    // Warm up: the buffer grows to what these reads need, and the same reads come again below.
    let expected = decode_pass(&mut dec, &stream, 9);
    assert_eq!(expected.0, 401);
    decode_pass(&mut dec, &stream, 9);

    let before = allocations();
    for _ in 0..50 {
        assert_eq!(decode_pass(&mut dec, &stream, 9), expected);
    }
    let made = allocations() - before;
    assert_eq!(made, 0, "decoding 50 passes of {} MB allocated {made} times", stream.len() / 1_000_000);
}

#[test]
fn hot_frames_encode_without_allocating() {
    let mut rng = Rng::new(2);
    let data = rng.bytes(MAX_DATA);
    let mut buf = Vec::new();
    let pass = |buf: &mut Vec<u8>| {
        for (i, len) in [1usize, 50, 1000, 20_000, MAX_DATA].into_iter().cycle().take(200).enumerate() {
            buf.clear();
            let bytes = &data[..len];
            let msg = if i % 3 == 0 {
                Message::Input(bytes)
            } else {
                Message::Out(Out { offset: i as u64, bytes })
            };
            encode(buf, 1, 0, &msg).expect("encodes");
        }
    };
    pass(&mut buf);
    let before = allocations();
    for _ in 0..100 {
        pass(&mut buf);
    }
    assert_eq!(allocations() - before, 0);
}

#[test]
fn every_message_encodes_without_allocating() {
    let all = samples::all();
    let mut buf = Vec::new();
    for (_, msg) in &all {
        buf.clear();
        encode(&mut buf, 1, 0, msg).expect("encodes");
    }
    let before = allocations();
    for _ in 0..100 {
        for (_, msg) in &all {
            buf.clear();
            encode(&mut buf, 1, 0, msg).expect("encodes");
        }
    }
    assert_eq!(allocations() - before, 0);
}

#[test]
fn the_decoder_gives_back_a_rare_huge_frames_memory() {
    let big = vec![1u8; MAX_PAYLOAD - 100];
    let mut stream = Vec::new();
    encode(&mut stream, 1, 0, &Message::Hook(Hook { name: "h", payload: &big[..MAX_PAYLOAD - 200] }))
        .expect("encodes");
    let mut dec = Decoder::new();
    dec.push(&stream);
    assert!(dec.next_frame().expect("valid").is_some());
    assert!(dec.capacity() > MAX_PAYLOAD / 2, "it held the whole frame");
    let small = encode_one(1, &Message::Ping(1));
    dec.push(&small);
    assert!(dec.next_frame().expect("ok").is_some());
    // The next read finds it empty and large, and lets go.
    let mut src = Chunked { data: &small, step: || 100 };
    dec.fill(&mut src).expect("reads");
    assert!(dec.capacity() < MAX_PAYLOAD / 2, "capacity {} was kept", dec.capacity());
}
