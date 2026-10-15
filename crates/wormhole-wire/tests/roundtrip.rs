//! Every message survives encode then decode, however the bytes are cut, and the encoder and
//! decoder keep their limits at the boundaries.

mod common;

use common::*;
use wormhole_wire::*;

fn decode_one(bytes: &[u8]) -> String {
    let mut dec = Decoder::new();
    dec.push(bytes);
    let frame = dec.next_frame().expect("decodes").expect("a frame");
    let text = format!("{frame:?}");
    assert!(dec.next_frame().expect("no error").is_none());
    assert!(dec.is_idle(), "nothing left over");
    text
}

#[test]
fn every_sample_round_trips() {
    for (name, msg) in samples::all() {
        let bytes = encode_one(7, &msg);
        let header = Header::parse(bytes[..HEADER_LEN].try_into().expect("8 bytes"));
        assert_eq!(header.len as usize, bytes.len() - HEADER_LEN, "{name}: len counts the payload");
        assert_eq!(header.ty, msg.ty(), "{name}");
        assert_eq!(header.channel, 7, "{name}");
        assert!(header.len as usize <= max_payload(header.ty).expect("known"), "{name}");

        let mut dec = Decoder::new();
        dec.push(&bytes);
        let frame = dec.next_frame().expect(name).expect(name);
        assert_eq!(frame.message, msg, "{name}");
        assert_eq!(frame.channel, 7);
        assert_eq!(decode_one(&bytes), format!("{frame:?}"));
    }
}

#[test]
fn every_sample_decodes_from_single_bytes() {
    let all = samples::all();
    let mut stream = Vec::new();
    for (_, msg) in &all {
        encode(&mut stream, 1, 0, msg).expect("encodes");
    }
    let whole = decode_all(&stream, || usize::MAX);
    assert_eq!(whole.len(), all.len());
    assert_eq!(decode_all(&stream, || 1), whole, "one byte at a time");
    let mut rng = Rng::new(5);
    assert_eq!(decode_all(&stream, move || 1 + rng.usize(40)), whole, "random cuts");
}

#[test]
fn random_messages_round_trip() {
    for seed in 1..=8u64 {
        let mut rng = Rng::new(seed);
        let pool = Pool::new(&mut rng);
        for case in 0..3000 {
            let msg = message(&mut rng, &pool);
            let channel = rng.u16();
            let mut buf = Vec::new();
            encode(&mut buf, channel, flags::URGENT * u8::from(rng.bool()), &msg)
                .unwrap_or_else(|e| panic!("seed {seed} case {case}: {msg:?} does not encode: {e}"));
            let mut dec = Decoder::new();
            dec.push(&buf);
            let frame = dec
                .next_frame()
                .unwrap_or_else(|e| panic!("seed {seed} case {case}: {e} for {msg:?}"))
                .unwrap_or_else(|| panic!("seed {seed} case {case}: no frame for {msg:?}"));
            assert_eq!(frame.message, msg, "seed {seed} case {case}");
            assert_eq!(frame.channel, channel);
        }
    }
}

#[test]
fn random_streams_decode_the_same_however_they_are_cut() {
    for seed in 1..=6u64 {
        let mut rng = Rng::new(100 + seed);
        let pool = Pool::new(&mut rng);
        let mut stream = Vec::new();
        let mut expected = Vec::new();
        for _ in 0..400 {
            let msg = message(&mut rng, &pool);
            encode(&mut stream, 1, 0, &msg).expect("encodes");
            expected.push(format!("{:?}", Frame { channel: 1, flags: 0, message: msg }));
        }
        assert_eq!(decode_all(&stream, || usize::MAX), expected, "seed {seed} whole");
        let mut cuts = Rng::new(seed);
        let pieces = decode_all(&stream, move || 1 + cuts.usize(70_000));
        assert_eq!(pieces, expected, "seed {seed} random cuts");
        let mut small = Rng::new(seed + 7);
        assert_eq!(decode_all(&stream, move || 1 + small.usize(9)), expected, "seed {seed} tiny cuts");
    }
}

#[test]
fn the_header_is_eight_bytes_little_endian() {
    let msg = Message::Out(Out { offset: 0x0102_0304_0506_0708, bytes: b"xy" });
    let bytes = {
        let mut b = Vec::new();
        encode(&mut b, 0xABCD, flags::URGENT, &msg).expect("encodes");
        b
    };
    assert_eq!(&bytes[..8], &[10, 0, 0, 0, ty::OUT, flags::URGENT, 0xCD, 0xAB]);
    assert_eq!(&bytes[8..16], &[8, 7, 6, 5, 4, 3, 2, 1], "offset, little-endian");
    assert_eq!(&bytes[16..], b"xy");
}

#[test]
fn the_frozen_bootstrap_prefix_is_64_bytes() {
    let (_, hello) = samples::all().into_iter().find(|(n, _)| *n == "hello_controller").expect("sample");
    let b = encode_one(1, &hello);
    // 8 header + 56 fixed, then the TLV tail (the client name).
    assert_eq!(&b[8..12], b"WHSP");
    assert_eq!(&b[12..14], &1u16.to_le_bytes(), "proto_min");
    assert_eq!(&b[14..16], &1u16.to_le_bytes(), "proto_max");
    assert_eq!(b[18], 1, "role: controller");
    assert!(b[8 + 26..64].iter().all(|&x| x == 0), "reserved bytes are zero");
    assert_eq!(b[64], 1, "first TLV tag: client name");

    let (_, welcome) = samples::all().into_iter().find(|(n, _)| *n == "welcome_running").expect("sample");
    let b = encode_one(1, &welcome);
    assert_eq!(&b[8..12], b"WHSP");
    assert_eq!(&b[12..14], &1u16.to_le_bytes(), "proto");
    assert_eq!(&b[28..40], b"abcdefghijkl", "session id at 20 in the payload");
    assert_eq!(b[64], 1, "first TLV tag: size");
}

#[test]
fn data_frames_hold_exactly_one_chunk() {
    let chunk = vec![0xA5; MAX_DATA];
    for msg in [
        Message::Out(Out { offset: 1, bytes: &chunk }),
        Message::Input(&chunk),
        Message::Page(Page { epoch: 1, from: 2, count: 3, bytes: &chunk }),
        Message::Repaint(Repaint {
            epoch: 1,
            offset: 2,
            kind: RepaintKind::Full,
            history_gap: false,
            first: true,
            last: true,
            bytes: &chunk,
        }),
    ] {
        let bytes = encode_one(1, &msg);
        assert_eq!(decode_one(&bytes), format!("{:?}", Frame { channel: 1, flags: 0, message: msg }));
    }
}

#[test]
fn an_oversize_message_is_refused_and_leaves_the_buffer_alone() {
    let big = vec![0; MAX_DATA + 1];
    let mut buf = b"keep".to_vec();
    for msg in [
        Message::Out(Out { offset: 0, bytes: &big }),
        Message::Input(&big),
        Message::Page(Page { epoch: 0, from: 0, count: 0, bytes: &big }),
    ] {
        assert!(matches!(encode(&mut buf, 1, 0, &msg), Err(EncodeError::DataTooLarge(_))), "{msg:?}");
        assert_eq!(buf, b"keep");
    }
    let long = "x".repeat(70_000);
    let msg = Message::Error(ErrorMsg { ref_type: 0, code: 0, text: &long });
    assert!(matches!(encode(&mut buf, 1, 0, &msg), Err(EncodeError::StringTooLong(_))));
    assert_eq!(buf, b"keep");
    // Compression is negotiated and not implemented: refused, not silently ignored.
    let ping = Message::Ping(1);
    assert!(encode(&mut buf, 1, flags::COMPRESSED, &ping).is_err());
    assert_eq!(buf, b"keep");
}

#[test]
fn encoding_appends_to_what_is_there() {
    let mut buf = Vec::new();
    encode(&mut buf, 1, 0, &Message::Ping(1)).expect("encodes");
    let first = buf.len();
    encode(&mut buf, 1, 0, &Message::Pong { n: 1, pty_progress: 2 }).expect("encodes");
    assert!(buf.len() > first);
    assert_eq!(decode_all(&buf, || usize::MAX).len(), 2);
}

#[test]
fn lists_are_bounded_on_both_sides() {
    let too_many: Vec<&str> = vec!["a"; MAX_SPAWN_ITEMS + 1];
    let spawn = |args: Vec<&'static str>| {
        Message::Spawn(Spawn {
            size: Size { cols: 1, rows: 1, cell_w: 1, cell_h: 1 },
            options: Options { scrollback_lines: 0, flags: 0 },
            program: "p",
            cwd: "",
            args: args.into(),
            env: Vec::new().into(),
            colors: None,
        })
    };
    assert!(matches!(encode(&mut Vec::new(), 1, 0, &spawn(too_many)), Err(EncodeError::TooMany(_))));
    let ok: Vec<&str> = vec!["a"; MAX_SPAWN_ITEMS];
    assert_eq!(decode_all(&encode_one(1, &spawn(ok)), || usize::MAX).len(), 1);
}
