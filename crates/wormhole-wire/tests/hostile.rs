//! The decoder reads what an attacker or a bug sends: lengths that lie, payloads cut short or
//! garbled, streams of noise. It must never panic, never buffer past its bound and always say
//! what was wrong with a typed error.

mod support;

use support::*;
use wormhole_wire::*;

/// The most a decoder may hold: one maximum frame, its header and one more read.
const BOUND: usize = MAX_PAYLOAD + 2 * (MAX_DATA + HEADER_LEN);

#[test]
fn a_length_past_the_maximum_is_fatal_before_anything_is_buffered() {
    for ty in [ty::SPAWN, ty::PING, ty::OUT, ty::INPUT, ty::HELLO, 0x7F] {
        let mut dec = Decoder::new();
        // Only the header: nothing of the announced payload has arrived.
        let h = Header { len: MAX_PAYLOAD as u32 + 1, ty, flags: 0, channel: 1 }.to_bytes();
        dec.push(&h);
        let err = dec.next_frame().expect_err("rejected");
        assert!(matches!(err, WireError::FrameTooLarge { .. }), "type {ty:#x}: {err:?}");
        assert!(err.is_fatal());
        assert!(dec.capacity() < 2 * (MAX_DATA + HEADER_LEN), "nothing was reserved for the payload");
    }
}

#[test]
fn a_big_announcement_reserves_nothing_until_the_bytes_arrive() {
    let header = Header { len: MAX_PAYLOAD as u32, ty: ty::SPAWN, flags: 0, channel: 1 }.to_bytes();
    let mut dec = Decoder::new();
    let mut src = Chunked { data: &header, step: || 8 };
    dec.fill(&mut src).expect("reads");
    assert!(dec.next_frame().expect("a valid header").is_none());
    // The next read asks for a chunk, not for the megabyte the header promised.
    let mut src = Chunked { data: &[0u8; 100], step: || 100 };
    dec.fill(&mut src).expect("reads");
    assert!(dec.capacity() < 4 * MAX_DATA, "reserved {} bytes for a peer that sent 108", dec.capacity());
}

#[test]
fn each_type_has_its_own_maximum() {
    // A control message may not claim a data frame's size, nor a data frame a megabyte.
    let too_big = |ty, len: usize| {
        let h = Header { len: len as u32, ty, flags: 0, channel: 1 }.to_bytes();
        let mut dec = Decoder::new();
        dec.push(&h);
        matches!(dec.next_frame(), Err(WireError::FrameTooLarge { .. }))
    };
    assert!(too_big(ty::PING, MAX_CONTROL + 1));
    assert!(!too_big(ty::PING, MAX_CONTROL));
    assert!(too_big(ty::OUT, OUT_PREFIX + MAX_DATA + 1));
    assert!(!too_big(ty::OUT, OUT_PREFIX + MAX_DATA));
    assert!(too_big(ty::INPUT, MAX_DATA + 1));
    assert!(too_big(ty::REPAINT, REPAINT_PREFIX + MAX_DATA + 1));
    assert!(too_big(ty::PAGE, PAGE_PREFIX + MAX_DATA + 1));
    assert!(!too_big(ty::SPAWN, MAX_PAYLOAD));
    assert!(too_big(ty::SPAWN, MAX_PAYLOAD + 1));
}

#[test]
fn before_the_handshake_only_small_frames_are_allowed() {
    let mut dec = Decoder::new();
    dec.limit_to_handshake();
    dec.push(&Header { len: MAX_CONTROL as u32 + 1, ty: ty::SPAWN, flags: 0, channel: 1 }.to_bytes());
    assert!(
        matches!(dec.next_frame(), Err(WireError::FrameTooLarge { max, .. }) if max as usize == MAX_CONTROL)
    );

    // And an unknown type obeys the same bound.
    let mut dec = Decoder::new();
    dec.limit_to_handshake();
    dec.push(&Header { len: MAX_CONTROL as u32 + 1, ty: 0x7F, flags: 0, channel: 1 }.to_bytes());
    assert!(dec.next_frame().is_err());
}

#[test]
fn a_fatal_error_ends_the_stream_for_good() {
    let mut dec = Decoder::new();
    dec.push(&Header { len: u32::MAX, ty: ty::OUT, flags: 0, channel: 1 }.to_bytes());
    let err = dec.next_frame().expect_err("fatal");
    assert!(dec.is_failed());
    // Valid frames after it are not trusted: the framing is lost.
    dec.push(&encode_one(1, &Message::Ping(1)));
    assert_eq!(dec.next_frame().expect_err("still fatal"), err);
    assert_eq!(dec.next_frame().expect_err("still fatal"), err);
}

#[test]
fn compression_that_was_not_negotiated_is_fatal() {
    let frame = raw_frame(ty::PING, flags::COMPRESSED, 1, &[0; 8]);
    assert_eq!(first(&frame), Err(WireError::Compressed));
}

#[test]
fn unknown_flag_bits_are_advice_and_ignored() {
    let frame = raw_frame(ty::PING, 0b1111_1100 | flags::URGENT, 1, &9u64.to_le_bytes());
    let mut dec = Decoder::new();
    dec.push(&frame);
    let f = dec.next_frame().expect("decodes").expect("frame");
    assert_eq!(f.message, Message::Ping(9));
}

#[test]
fn payload_errors_are_typed_and_leave_the_stream_in_step() {
    let ping = encode_one(1, &Message::Ping(5));
    let bad: Vec<(Vec<u8>, WireError)> = vec![
        (raw_frame(ty::PING, 0, 1, &[1, 2, 3]), WireError::Truncated("Ping")),
        (raw_frame(ty::HELLO, 0, 1, b"WHSP\0\0\0\0"), WireError::Truncated("Hello")),
        (raw_frame(ty::HELLO, 0, 1, &[b'N'; 56]), WireError::BadMagic),
        (raw_frame(ty::WELCOME, 0, 1, &[b'X'; 80]), WireError::BadMagic),
        (raw_frame(ty::INCOMPATIBLE, 0, 1, b"WHSP"), WireError::Truncated("Incompatible")),
        (
            raw_frame(ty::EVENT, 0, 1, &{
                let mut p = 1u64.to_le_bytes().to_vec();
                p.extend_from_slice(&[1, 2, 0, 0xFF, 0xFE]);
                p
            }),
            WireError::BadUtf8("Event"),
        ),
        (raw_frame(ty::OUT, 0, 1, &[0; 7]), WireError::Truncated("Out")),
        (raw_frame(ty::REPAINT, 0, 1, &[0; 15]), WireError::Truncated("Repaint")),
        // A middle part with no bytes at all is nothing a sender produces.
        (raw_frame(ty::REPAINT, 0, 1, &[0; 16]), WireError::BadValue("Repaint")),
        // A bool that is neither 0 nor 1.
        (
            raw_frame(ty::COUNTERS, 0, 1, &{
                let mut p = vec![0; 27];
                p[18] = 2;
                p
            }),
            WireError::BadValue("Counters"),
        ),
        (raw_frame(ty::FIND_START, 0, 1, &[0; 5]), WireError::Truncated("FindStart")),
    ];
    for (frame, want) in bad {
        let mut dec = Decoder::new();
        dec.push(&frame);
        dec.push(&ping);
        assert_eq!(dec.next_frame().expect_err("rejected"), want);
        assert!(!want.is_fatal());
        // The bad frame was consumed whole; the next one reads.
        let next = dec.next_frame().expect("in step").expect("the ping");
        assert_eq!(next.message, Message::Ping(5));
    }
}

#[test]
fn hostile_version_ranges_are_refused() {
    let hello = |min: u16, max: u16| {
        let (_, m) = samples::all().into_iter().find(|(n, _)| *n == "hello_controller").expect("sample");
        let mut p = payload_of(&encode_one(1, &m)).to_vec();
        p[4..6].copy_from_slice(&min.to_le_bytes());
        p[6..8].copy_from_slice(&max.to_le_bytes());
        raw_frame(ty::HELLO, 0, 1, &p)
    };
    assert_eq!(first(&hello(0, 1)), Err(WireError::BadValue("Hello")));
    assert_eq!(first(&hello(3, 2)), Err(WireError::BadValue("Hello")));
    assert!(first(&hello(2, 2)).expect("ok").is_some());
}

#[test]
fn tlv_tails_are_checked() {
    let hello = encode_one(
        1,
        &Message::Hello(Hello {
            proto_min: 1,
            proto_max: 1,
            proto_minor: 0,
            role: Role::Observer,
            force: false,
            features: Features::NONE,
            client_version: AppVersion::default(),
            client_name: "n",
            ticket: None,
        }),
    );
    let with = |tail: &[u8]| {
        let mut p = payload_of(&hello).to_vec();
        p.extend_from_slice(tail);
        raw_frame(ty::HELLO, 0, 1, &p)
    };
    // A length that runs off the end.
    assert_eq!(first(&with(&[9, 200, 0, 1])), Err(WireError::Truncated("Hello")));
    // A tag byte with nothing after it.
    assert_eq!(first(&with(&[9])), Err(WireError::Truncated("Hello")));
    // A known field twice.
    assert_eq!(first(&with(&[1, 1, 0, b'x'])), Err(WireError::DuplicateField("Hello")));
    // A known field that is not UTF-8.
    let mut p = payload_of(&hello).to_vec();
    let at = p.len() - 1;
    p[at] = 0xFF;
    assert_eq!(first(&raw_frame(ty::HELLO, 0, 1, &p)), Err(WireError::BadUtf8("Hello")));
    // An unknown field is fine.
    assert!(first(&with(&[200, 2, 0, 1, 2])).expect("ok").is_some());
}

#[test]
fn welcome_needs_its_size_and_counters() {
    let (_, w) = samples::all().into_iter().find(|(n, _)| *n == "welcome_running").expect("sample");
    let full = encode_one(1, &w);
    // The fixed part alone, without the TLVs.
    let fixed = raw_frame(ty::WELCOME, 0, 1, &payload_of(&full)[..BOOT_FIXED]);
    assert_eq!(first(&fixed), Err(WireError::MissingField("Welcome")));
}

#[test]
fn counts_that_lie_cannot_make_the_decoder_allocate() {
    // A Spawn that says it has 65,535 arguments in a few bytes.
    let mut p = vec![0; 8 + 8];
    p.extend_from_slice(&[0, 0, 0, 0]); // program, cwd: empty
    p.extend_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(first(&raw_frame(ty::SPAWN, 0, 1, &p)), Err(WireError::TooMany("Spawn")));
    // 4,096 of them, claimed, but only two present.
    let mut p = vec![0; 16];
    p.extend_from_slice(&[0, 0, 0, 0]);
    p.extend_from_slice(&(MAX_SPAWN_ITEMS as u16).to_le_bytes());
    p.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(first(&raw_frame(ty::SPAWN, 0, 1, &p)), Err(WireError::Truncated("Spawn")));
    // Backlog and find results.
    let p = (MAX_BACKLOG as u16 + 1).to_le_bytes();
    assert_eq!(first(&raw_frame(ty::BACKLOG, 0, 1, &p)), Err(WireError::TooMany("Backlog")));
    let mut p = vec![0; 9];
    p.extend_from_slice(&(MAX_MATCHES_PER_FRAME as u16 + 1).to_le_bytes());
    assert_eq!(first(&raw_frame(ty::FIND_RESULTS, 0, 1, &p)), Err(WireError::TooMany("FindResults")));
    let mut p = vec![0; 10];
    p.extend_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(first(&raw_frame(ty::SET_COLORS, 0, 1, &p)), Err(WireError::TooMany("Colors")));
}

#[test]
fn an_unknown_giant_frame_is_skipped_without_buffering_it() {
    let mut dec = Decoder::new();
    dec.push(&Header { len: MAX_PAYLOAD as u32, ty: 0x7F, flags: 0, channel: 1 }.to_bytes());
    let chunk = [0u8; 1000];
    let mut sent = 0;
    while sent < MAX_PAYLOAD {
        let n = chunk.len().min(MAX_PAYLOAD - sent);
        dec.push(&chunk[..n]);
        sent += n;
        assert!(dec.next_frame().expect("skipping").is_none());
        assert!(dec.buffered() < 2 * chunk.len(), "the skipped payload is dropped as it streams");
    }
    dec.push(&encode_one(1, &Message::Ping(3)));
    let f = dec.next_frame().expect("ok").expect("the ping after it");
    assert_eq!(f.message, Message::Ping(3));
    assert_eq!(dec.skipped(), (1, MAX_PAYLOAD as u64));
}

#[test]
fn a_trickled_maximum_frame_stays_within_the_bound() {
    let big = vec![b'x'; MAX_PAYLOAD - 100];
    // A Hook is the largest known shape with free-form bytes; build a valid one near the maximum.
    let hook = Message::Hook(Hook { name: "h", payload: &big[..MAX_PAYLOAD - 200] });
    let bytes = encode_one(1, &hook);
    let mut dec = Decoder::new();
    let mut seen = 0;
    for piece in bytes.chunks(777) {
        dec.push(piece);
        assert!(dec.capacity() <= BOUND);
        while let Some(f) = dec.next_frame().expect("valid") {
            assert!(matches!(f.message, Message::Hook(_)));
            seen += 1;
        }
    }
    assert_eq!(seen, 1);
}

#[test]
fn noise_never_panics_and_stays_bounded() {
    for seed in 0..40u64 {
        let mut rng = Rng::new(seed);
        for _ in 0..300 {
            let len = rng.usize(400);
            let mut noise = rng.bytes(len);
            // Make a share of them look like frames, so the payload decoders get exercised.
            if rng.bool() && noise.len() >= HEADER_LEN {
                noise[4] =
                    [ty::HELLO, ty::WELCOME, ty::SPAWN, ty::EVENT, ty::BACKLOG, ty::FIND_RESULTS, 0x7F]
                        [rng.usize(7)];
                noise[5] &= 0b10;
                let l = (noise.len() - HEADER_LEN) as u32;
                noise[0..4].copy_from_slice(&l.to_le_bytes());
            }
            let mut step = Rng::new(seed ^ rng.next());
            let _ = decode_all(&noise, move || 1 + step.usize(50));
        }
    }
}

#[test]
fn mutated_valid_streams_never_panic() {
    let mut stream = Vec::new();
    for (_, msg) in samples::all() {
        encode(&mut stream, 1, 0, &msg).expect("encodes");
    }
    let mut rng = Rng::new(77);
    for _ in 0..3000 {
        let mut s = stream.clone();
        match rng.below(4) {
            0 => {
                for _ in 0..1 + rng.usize(8) {
                    let at = rng.usize(s.len());
                    s[at] = rng.u8();
                }
            }
            1 => s.truncate(rng.usize(s.len())),
            2 => {
                let at = rng.usize(s.len());
                s.drain(at..(at + rng.usize(30)).min(s.len()));
            }
            _ => {
                let at = rng.usize(s.len());
                let junk = rng.noise(20);
                s.splice(at..at, junk);
            }
        }
        let mut step = Rng::new(rng.next());
        let _ = decode_all(&s, move || 1 + step.usize(3000));
    }
}

#[test]
fn any_message_payload_decodes_or_errors_never_panics() {
    // Every type id with random payloads, straight into the message decoder.
    let mut rng = Rng::new(9);
    for t in 0..=255u8 {
        for _ in 0..400 {
            let payload = rng.noise(120);
            let _ = Message::decode(t, &payload);
        }
    }
}

#[test]
fn cutting_a_stream_never_changes_what_it_means() {
    // A stream that ends in an error reports it at the same frame however it is cut.
    let mut stream = Vec::new();
    for (_, msg) in samples::all().into_iter().take(20) {
        encode(&mut stream, 1, 0, &msg).expect("encodes");
    }
    stream.extend_from_slice(&raw_frame(ty::HELLO, 0, 1, &[b'N'; 56]));
    stream.extend_from_slice(&encode_one(1, &Message::Ping(1)));
    let whole = decode_all(&stream, || usize::MAX);
    assert!(whole.iter().any(|l| l.starts_with("error BadMagic")));
    assert_eq!(decode_all(&stream, || 1), whole);
}
