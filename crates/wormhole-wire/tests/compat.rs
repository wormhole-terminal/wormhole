//! Compatibility: a peer of minor N reads what a peer of minor N+1 sends, by skipping what it
//! added; and majors are negotiated by range. The messages of "minor N+1" below are built by hand
//! with a frame type, a TLV tag, an event kind and enum values this version has never heard of.

mod support;

use support::*;
use wormhole_wire::version::{HostFit, PROTO_MAJOR, Range};
use wormhole_wire::*;

/// A frame with extra TLVs after its payload, the way a newer minor adds a field.
fn with_tail(frame: &[u8], tail: &[u8]) -> Vec<u8> {
    let h = Header::parse(frame[..HEADER_LEN].try_into().expect("8 bytes"));
    let mut payload = frame[HEADER_LEN..].to_vec();
    payload.extend_from_slice(tail);
    raw_frame(h.ty, h.flags, h.channel, &payload)
}

const FUTURE_TLV: &[u8] = &[0xC8, 3, 0, 1, 2, 3];

#[test]
fn a_new_tag_in_a_known_message_is_ignored() {
    // Messages that end in a TLV tail.
    for name in
        ["hello_controller", "hello_admin_ticket", "welcome_running", "spawn", "attach_resume", "resize"]
    {
        let (_, msg) = samples::all().into_iter().find(|(n, _)| *n == name).expect("sample");
        let newer = with_tail(&encode_one(1, &msg), FUTURE_TLV);
        let mut dec = Decoder::new();
        dec.push(&newer);
        assert_eq!(dec.next_frame().expect(name).expect(name).message, msg, "{name}");
    }
}

#[test]
fn trailing_bytes_after_fixed_fields_are_ignored() {
    for name in [
        "ping",
        "pong",
        "ack",
        "input_ack",
        "counters",
        "resize_applied",
        "close",
        "detached",
        "find_cancel",
        "page_get",
        "set_limits",
    ] {
        let (_, msg) = samples::all().into_iter().find(|(n, _)| *n == name).expect("sample");
        let newer = with_tail(&encode_one(1, &msg), FUTURE_TLV);
        let mut dec = Decoder::new();
        dec.push(&newer);
        assert_eq!(dec.next_frame().expect(name).expect(name).message, msg, "{name}");
    }
}

#[test]
fn an_unknown_message_type_is_skipped_and_counted_wherever_the_stream_is_cut() {
    let mut stream = Vec::new();
    stream.extend(encode_one(1, &Message::Ping(1)));
    stream.extend(raw_frame(0x40, 0, 1, b"a message of minor 1"));
    stream.extend(raw_frame(0xEE, flags::URGENT, 9, &[]));
    stream.extend(encode_one(1, &Message::Ping(2)));
    stream.extend(raw_frame(0x41, 0, 1, &vec![7; 70_000]));
    stream.extend(encode_one(1, &Message::Ping(3)));

    let want = ["Ping(1)", "Ping(2)", "Ping(3)"];
    for step in [usize::MAX, 1, 3, 7, 4096] {
        let mut dec = Decoder::new();
        let mut src = Chunked { data: &stream, step: move || step };
        let mut got = Vec::new();
        loop {
            while let Some(f) = dec.next_frame().expect("no error") {
                got.push(format!("{:?}", f.message));
            }
            if dec.fill(&mut src).expect("reads") == 0 {
                break;
            }
        }
        assert_eq!(got, want, "step {step}");
        assert_eq!(dec.skipped(), (3, (20 + 70_000) as u64), "step {step}");
        assert_eq!(dec.frames(), 3);
    }
}

#[test]
fn enum_values_from_a_newer_minor_survive_as_unknown() {
    let (_, hello) = samples::all().into_iter().find(|(n, _)| *n == "hello_controller").expect("sample");
    let mut p = payload_of(&encode_one(1, &hello)).to_vec();
    p[10] = 42; // a role that does not exist yet
    match first(&raw_frame(ty::HELLO, 0, 1, &p)).expect("decodes") {
        Some(text) => assert!(text.contains("Unknown(42)"), "{text}"),
        None => panic!("no frame"),
    }

    let attach = Message::Attach(Attach {
        stream: StreamMode::Unknown(9),
        history_lines: 1,
        window: 2,
        resume_from: None,
    });
    let bytes = encode_one(1, &attach);
    let mut dec = Decoder::new();
    dec.push(&bytes);
    assert_eq!(dec.next_frame().expect("ok").expect("frame").message, attach, "and it re-encodes as it came");
}

#[test]
fn an_event_kind_from_a_newer_minor_is_skippable() {
    let mut p = 77u64.to_le_bytes().to_vec();
    p.extend_from_slice(&[99, 1, 2, 3, 4]);
    let mut dec = Decoder::new();
    dec.push(&raw_frame(ty::EVENT, 0, 1, &p));
    let f = dec.next_frame().expect("ok").expect("frame");
    assert_eq!(f.message, Message::Event(Event { offset: 77, kind: EventKind::Unknown(99) }));

    // Inside a backlog too, between events this version does read.
    let mut entries = vec![];
    for body in [
        encode_one(1, &Message::Event(Event { offset: 1, kind: EventKind::Bell })),
        raw_frame(ty::EVENT, 0, 1, &p),
        encode_one(1, &Message::Event(Event { offset: 3, kind: EventKind::ResetTitle })),
    ] {
        let payload = payload_of(&body);
        entries.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        entries.extend_from_slice(payload);
    }
    let mut backlog = 3u16.to_le_bytes().to_vec();
    backlog.extend(entries);
    let mut dec = Decoder::new();
    dec.push(&raw_frame(ty::BACKLOG, 0, 1, &backlog));
    let Message::Backlog(events) = dec.next_frame().expect("ok").expect("frame").message else {
        panic!("a backlog");
    };
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].kind, EventKind::Unknown(99));
    assert_eq!(events[2].kind, EventKind::ResetTitle);
}

#[test]
fn a_whole_conversation_of_a_newer_minor_reads_on_this_one() {
    // What a minor-1 host might say: a Welcome with a new field, a message type we lack, an event
    // kind we lack, a ping, and output.
    let (_, welcome) = samples::all().into_iter().find(|(n, _)| *n == "welcome_running").expect("sample");
    let mut stream = with_tail(&encode_one(1, &welcome), FUTURE_TLV);
    stream.extend(raw_frame(0x50, flags::URGENT, 1, b"new in 1.1"));
    let mut ev = 5u64.to_le_bytes().to_vec();
    ev.push(250);
    stream.extend(raw_frame(ty::EVENT, 0, 1, &ev));
    stream.extend(encode_one(1, &Message::Out(Out { offset: 0, bytes: b"hi" })));

    let lines = decode_all(&stream, || usize::MAX);
    assert_eq!(lines.len(), 3, "{lines:#?}");
    assert!(lines[0].contains("Welcome"));
    assert!(lines[1].contains("Unknown(250)"));
    assert!(lines[2].contains("Out"));
}

#[test]
fn majors_are_negotiated_by_range() {
    assert_eq!(Range::new(1, 3).agree(Range::new(2, 5)), Some(3));
    assert_eq!(Range::new(1, 1).agree(Range::new(1, 1)), Some(1));
    assert_eq!(Range::new(4, 5).agree(Range::new(1, 3)), None);
    assert_eq!(Range::new(1, 2).agree(Range::new(3, 4)), None);

    let current = Range::current();
    assert!(current.min >= 1 && current.min <= current.max);
    assert_eq!(current.max, PROTO_MAJOR);
}

#[test]
fn the_two_majors_back_rule() {
    // A client of major 5 speaks 4 and 5.
    let client = Range::new(4, 5);
    assert_eq!(client.fit(Range::new(5, 5)), HostFit::Compatible(5));
    assert_eq!(client.fit(Range::new(4, 4)), HostFit::Compatible(4));
    assert_eq!(client.fit(Range::new(1, 4)), HostFit::Compatible(4));
    // A host of major 3 is older than the app keeps a codec for: read-only, admin still works.
    assert_eq!(client.fit(Range::new(3, 3)), HostFit::TooOld);
    assert_eq!(client.fit(Range::new(1, 2)), HostFit::TooOld);
    // A host that wants major 6 or later needs a newer app.
    assert_eq!(client.fit(Range::new(6, 7)), HostFit::TooNew);
}

#[test]
fn a_host_answers_a_hello_with_a_major_or_incompatible() {
    let hello = |min, max| Hello {
        proto_min: min,
        proto_max: max,
        proto_minor: 0,
        role: Role::Controller,
        force: false,
        features: Features::NONE,
        client_version: AppVersion::default(),
        client_name: "",
        ticket: None,
    };
    let host = Range::new(2, 3);
    let v = AppVersion { major: 1, minor: 0, patch: 0 };
    assert_eq!(host.answer(&hello(1, 2), v), Ok(2));
    assert_eq!(host.answer(&hello(3, 9), v), Ok(3));
    assert_eq!(host.answer(&hello(4, 9), v), Err(Incompatible { host_min: 2, host_max: 3, host_version: v }));
    // The admin hello offers every major, so it always finds one.
    assert!(host.answer(&Hello { proto_min: Range::admin().min, proto_max: 3, ..hello(1, 1) }, v).is_ok());
}

#[test]
fn the_bootstrap_is_the_same_whatever_the_role() {
    // An admin hello and a controller hello differ in one byte of the frozen prefix and are the same length.
    let hello = |role| {
        encode_one(
            1,
            &Message::Hello(Hello {
                proto_min: 1,
                proto_max: 1,
                proto_minor: 0,
                role,
                force: false,
                features: Features::NONE,
                client_version: AppVersion::default(),
                client_name: "",
                ticket: None,
            }),
        )
    };
    let (a, c) = (hello(Role::Admin), hello(Role::Controller));
    assert_eq!(a.len(), 64);
    assert_eq!(a.len(), c.len());
    let differing: Vec<_> = (0..a.len()).filter(|&i| a[i] != c[i]).collect();
    assert_eq!(differing, vec![18], "only the role byte");
}

#[test]
fn the_generator_reaches_every_message_type() {
    let mut seen = std::collections::BTreeSet::new();
    let mut rng = Rng::new(3);
    let pool = Pool::new(&mut rng);
    for _ in 0..5000 {
        seen.insert(message(&mut rng, &pool).ty());
    }
    let known = samples::all().into_iter().map(|(_, m)| m.ty()).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(seen, known, "the property tests cover every message the samples do");
}
