//! The codec over `Read` and `Write` (the seam a Unix socket and SSH stdio both plug into), and
//! the ordering checks a receiver runs on a stream.

mod support;

use std::io::{self, Read, Write};

use support::*;
use wormhole_wire::io::{FrameReader, FrameWriter, RecvError};
use wormhole_wire::*;

#[test]
fn frames_flow_through_any_read_and_write() {
    let mut wire = Vec::new();
    {
        let mut w = FrameWriter::new(&mut wire);
        for (_, msg) in samples::all() {
            w.send(CHANNEL_LOCAL, 0, &msg).expect("sends");
        }
    }
    let mut r = FrameReader::new(&wire[..]);
    let mut n = 0;
    while let Some(frame) = r.read_frame().expect("reads") {
        assert_eq!(frame.channel, CHANNEL_LOCAL);
        n += 1;
    }
    assert_eq!(n, samples::all().len(), "and a clean end of stream is None");
}

#[test]
fn a_stream_that_ends_inside_a_frame_is_an_error_not_a_clean_end() {
    let bytes = encode_one(1, &Message::Ping(1));
    for cut in 1..bytes.len() {
        let mut r = FrameReader::new(&bytes[..cut]);
        assert!(matches!(r.read_frame(), Err(RecvError::UnexpectedEof)), "cut at {cut}");
    }
    let mut r = FrameReader::new(&bytes[..0]);
    assert!(matches!(r.read_frame(), Ok(None)));
}

/// Fails the way real sockets do before they succeed.
struct Flaky<'a> {
    data: &'a [u8],
    script: Vec<io::ErrorKind>,
}

impl Read for Flaky<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(kind) = self.script.pop() {
            return Err(kind.into());
        }
        let n = buf.len().min(self.data.len()).min(3);
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

#[test]
fn interrupted_reads_are_retried_and_would_block_is_reported() {
    let bytes = encode_one(1, &Message::Ping(8));
    let mut r = FrameReader::new(Flaky {
        data: &bytes,
        script: vec![io::ErrorKind::Interrupted, io::ErrorKind::Interrupted],
    });
    let f = r.read_frame().expect("retried").expect("a frame");
    assert_eq!(f.message, Message::Ping(8));

    let mut r = FrameReader::new(Flaky { data: &bytes, script: vec![io::ErrorKind::WouldBlock] });
    match r.read_frame() {
        Err(RecvError::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::WouldBlock),
        other => panic!("{other:?}"),
    }
    // The caller waits for readiness and calls again; nothing was lost.
    let f = r.read_frame().expect("continues").expect("a frame");
    assert_eq!(f.message, Message::Ping(8));
}

#[test]
fn a_fatal_wire_error_comes_out_of_the_reader() {
    let bytes = Header { len: u32::MAX, ty: ty::OUT, flags: 0, channel: 1 }.to_bytes();
    let mut r = FrameReader::new(&bytes[..]);
    match r.read_frame() {
        Err(RecvError::Wire(e)) => assert!(e.is_fatal()),
        other => panic!("{other:?}"),
    }
}

struct CountWrites {
    writes: usize,
    bytes: usize,
}

impl Write for CountWrites {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_batch_of_frames_is_one_write() {
    let mut w = FrameWriter::new(CountWrites { writes: 0, bytes: 0 });
    for i in 0..10 {
        w.queue(1, 0, &Message::Out(Out { offset: i * 100, bytes: &[0; 100] })).expect("queues");
    }
    w.queue(
        1,
        0,
        &Message::Counters(Counters {
            epoch: 0,
            history_lines: 0,
            trimmed_total: 0,
            screen_rows: 0,
            alt_screen: false,
            revision: 0,
        }),
    )
    .expect("queues");
    assert!(w.queued() > 1000);
    w.flush().expect("writes");
    assert_eq!(w.queued(), 0);
    let sink = w.into_inner();
    assert_eq!(sink.writes, 1);
    assert_eq!(sink.bytes, 10 * (HEADER_LEN + OUT_PREFIX + 100) + HEADER_LEN + 27);
}

#[cfg(unix)]
#[test]
fn a_conversation_over_a_unix_socket() {
    use std::os::unix::net::UnixStream;

    let (a, b) = UnixStream::pair().expect("socketpair");
    let host = std::thread::spawn(move || {
        let mut r = FrameReader::new(b.try_clone().expect("clone"));
        let mut w = FrameWriter::new(b);
        let mut echoed = 0u64;
        while let Some(frame) = r.read_frame().expect("reads") {
            match frame.message {
                Message::Input(bytes) => {
                    echoed += bytes.len() as u64;
                    w.send(
                        frame.channel,
                        0,
                        &Message::Out(Out { offset: echoed - bytes.len() as u64, bytes }),
                    )
                    .expect("echoes");
                }
                Message::Detach => break,
                other => panic!("{other:?}"),
            }
        }
        echoed
    });

    let mut r = FrameReader::new(a.try_clone().expect("clone"));
    let mut w = FrameWriter::new(a);
    let mut order = StreamOrder::new();
    for i in 0..200u32 {
        let line = format!("line {i}\r");
        w.send(1, flags::URGENT, &Message::Input(line.as_bytes())).expect("sends");
        let f = r.read_frame().expect("reads").expect("an echo");
        let Message::Out(out) = f.message else { panic!("{:?}", f.message) };
        assert_eq!(out.bytes, line.as_bytes());
        order.out(&out).expect("contiguous");
    }
    w.send(1, 0, &Message::Detach).expect("detaches");
    let total: u64 = (0..200u32).map(|i| format!("line {i}\r").len() as u64).sum();
    assert_eq!(host.join().expect("host"), total);
    assert_eq!(order.next_offset(), Some(total));
}

fn out(offset: u64, len: usize) -> Out<'static> {
    const DATA: [u8; 64] = [0; 64];
    Out { offset, bytes: &DATA[..len] }
}

fn repaint(offset: u64, first: bool, last: bool) -> Repaint<'static> {
    Repaint { epoch: 1, offset, kind: RepaintKind::Full, history_gap: false, first, last, bytes: b"x" }
}

fn resized(offset: u64) -> ResizeApplied {
    ResizeApplied { offset, epoch: 2, req: 0, size: Size { cols: 1, rows: 1, cell_w: 1, cell_h: 1 } }
}

#[test]
fn output_must_be_contiguous() {
    let mut o = StreamOrder::new();
    o.out(&out(100, 10)).expect("the first frame sets the start");
    o.out(&out(110, 5)).expect("contiguous");
    assert_eq!(o.out(&out(120, 1)), Err(OrderError::Gap { expected: 115, got: 120 }));
    assert_eq!(o.out(&out(100, 1)), Err(OrderError::Gap { expected: 115, got: 100 }));
}

#[test]
fn a_repaint_resets_the_point_and_never_goes_backwards() {
    let mut o = StreamOrder::new();
    o.repaint(&repaint(500, true, true)).expect("attach");
    assert_eq!(o.next_offset(), Some(500));
    o.out(&out(500, 10)).expect("raw continues from the repaint");
    o.repaint(&repaint(900, true, true)).expect("a resync may skip ahead");
    o.out(&out(900, 1)).expect("and the stream goes on from there");
    assert_eq!(o.repaint(&repaint(10, true, true)), Err(OrderError::Backwards { at: 901, got: 10 }));
}

#[test]
fn a_multi_part_repaint_is_whole_or_an_error() {
    let mut o = StreamOrder::new();
    o.repaint(&repaint(10, true, false)).expect("first part");
    assert_eq!(o.out(&out(10, 1)), Err(OrderError::InsideRepaint), "no output inside a repaint");
    assert_eq!(o.resize_applied(&resized(10)), Err(OrderError::InsideRepaint));
    assert_eq!(o.repaint(&repaint(10, true, true)), Err(OrderError::BrokenRepaint), "a second start");
    o.repaint(&repaint(10, false, false)).expect("middle");
    assert_eq!(o.repaint(&repaint(11, false, true)), Err(OrderError::BrokenRepaint), "another offset");
    o.repaint(&repaint(10, false, true)).expect("last");
    o.out(&out(10, 4)).expect("output follows");

    let mut o = StreamOrder::new();
    assert_eq!(o.repaint(&repaint(10, false, true)), Err(OrderError::BrokenRepaint), "a part with no start");
}

#[test]
fn a_resize_arrives_exactly_between_the_bytes_it_was_applied_between() {
    let mut o = StreamOrder::new();
    o.out(&out(0, 10)).expect("output");
    o.resize_applied(&resized(10)).expect("applied after byte 9");
    o.out(&out(10, 5)).expect("output continues at the same offset");
    assert_eq!(o.resize_applied(&resized(20)), Err(OrderError::Gap { expected: 15, got: 20 }));
}
