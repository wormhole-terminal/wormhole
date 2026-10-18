//! The client against a scripted host on the other end of a socket pair: the attach sequence and
//! the repaint's reset, credit, the input window, attaching again after `TooSlow`, resizes that
//! wait for the host, events, and the end with and without the program's exit. No host process,
//! no PTY.

use std::io;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use wormhole_vt::parser::Parser;
use wormhole_vt::term::Terminal;
use wormhole_wire::io::{FrameReader, FrameWriter, RecvError};
use wormhole_wire::{self as wire, CHANNEL_LOCAL, Decoder, DetachReason, EventKind, Message, StreamMode};

use super::events::exit_of;
use super::open::hosted;
use super::socket::Joined;
use super::{ACK_EVERY, INPUT_WINDOW};
use crate::osc::{Mark, PromptKind};
use crate::types::Line;
use crate::{GridSize, Pty, PtyEvent, PtyEvents, StartOptions};

const PATIENCE: Duration = Duration::from_secs(5);
const GRID: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

/// What the scripted host read from the client.
#[derive(Clone, Debug, PartialEq)]
enum Got {
    Attach(StreamMode),
    Ack(u64),
    Input(Vec<u8>),
    Resize(wire::Size),
    ClipboardReply(u32, Vec<u8>),
    Close,
    Detach,
    Ping,
    Other(u8),
}

impl Got {
    fn of(msg: &Message<'_>) -> Self {
        match msg {
            Message::Attach(a) => Self::Attach(a.stream),
            Message::Ack(n) => Self::Ack(*n),
            Message::Input(bytes) => Self::Input(bytes.to_vec()),
            Message::Resize(r) => Self::Resize(r.size),
            Message::ClipboardReply(r) => Self::ClipboardReply(r.ticket, r.data.to_vec()),
            Message::Close { .. } => Self::Close,
            Message::Detach => Self::Detach,
            Message::Ping(_) => Self::Ping,
            other => Self::Other(other.ty()),
        }
    }
}

/// The host's end of the pair.
struct Script {
    reader: FrameReader<UnixStream>,
    writer: FrameWriter<UnixStream>,
}

impl Script {
    fn send(&mut self, msg: &Message<'_>) {
        self.writer.send(CHANNEL_LOCAL, 0, msg).expect("the client reads");
    }

    /// The next frame within `timeout`, if any.
    fn read(&mut self, timeout: Duration) -> Option<Got> {
        self.reader.get_ref().set_read_timeout(Some(timeout.max(Duration::from_millis(1)))).ok()?;
        match self.reader.read_frame() {
            Ok(Some(frame)) => Some(Got::of(&frame.message)),
            Ok(None) => None,
            Err(RecvError::Io(err))
                if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
            {
                None
            }
            Err(err) => panic!("the client's stream broke: {err}"),
        }
    }

    /// Read until `pred` holds; that frame.
    fn expect(&mut self, what: &str, mut pred: impl FnMut(&Got) -> bool) -> Got {
        let until = Instant::now() + PATIENCE;
        loop {
            assert!(Instant::now() < until, "no {what} came");
            if let Some(got) = self.read(Duration::from_millis(50))
                && pred(&got)
            {
                return got;
            }
        }
    }

    fn event(&mut self, offset: u64, kind: EventKind<'_>) {
        self.send(&Message::Event(wire::Event { offset, kind }));
    }
}

fn wire_size(cols: u16, rows: u16) -> wire::Size {
    wire::Size { cols, rows, cell_w: 128, cell_h: 256 }
}

/// A hosted `Pty` on one end of a socket pair and the scripted host on the other; its first
/// `Attach` asks for `first`.
fn connected(first: StreamMode) -> (Pty, PtyEvents, Script) {
    let (ours, theirs) = UnixStream::pair().expect("a socket pair");
    let joined = Joined { stream: ours, dec: Decoder::new() };
    let options = StartOptions { scrollback_lines: 1000, ..StartOptions::default() };
    let (pty, events) =
        hosted(joined, "abcdef123456", GRID, GRID, &options, first, std::process::id()).expect("a client");
    let write = theirs.try_clone().expect("a second handle");
    let mut script = Script { reader: FrameReader::new(theirs), writer: FrameWriter::new(write) };
    assert_eq!(script.expect("the attach", |g| matches!(g, Got::Attach(_))), Got::Attach(first));
    (pty, events, script)
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The next event for which `pred` holds.
fn next_event(events: &PtyEvents, what: &str, mut pred: impl FnMut(&PtyEvent) -> bool) -> PtyEvent {
    let until = Instant::now() + PATIENCE;
    loop {
        assert!(Instant::now() < until, "no {what} event came");
        match events.try_recv() {
            Some(event) if pred(&event) => return event,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
}

/// A host-side terminal of `cols` x `rows` after `text`, and its repaint.
fn repaint_of(cols: usize, rows: usize, text: &[u8]) -> (Vec<u8>, wormhole_vt::term::repaint::Seeds) {
    let mut term = Terminal::new(cols, rows, 1000);
    let mut parser = Parser::new();
    term.advance(&mut parser, text);
    let mut bytes = Vec::new();
    let made = term.repaint(&parser, &mut bytes).expect("between sequences");
    (bytes, made.seeds)
}

/// Send a repaint at `offset` as the host does: the size, the seeds, the parts.
fn send_repaint(
    script: &mut Script,
    offset: u64,
    size: wire::Size,
    bytes: &[u8],
    seeds: wormhole_vt::term::repaint::Seeds,
) {
    script.send(&Message::ResizeApplied(wire::ResizeApplied { offset, epoch: 0, req: 0, size }));
    let seeds = EventKind::RepaintSeeds {
        block: seeds.block,
        link_seq: seeds.link_seq,
        image_handle: seeds.image_handle,
    };
    script.event(offset, seeds);
    let middle = bytes.len() / 2;
    for (i, part) in [&bytes[..middle], &bytes[middle..]].into_iter().enumerate() {
        script.send(&Message::Repaint(wire::Repaint {
            epoch: 0,
            offset,
            kind: wire::RepaintKind::Full,
            history_gap: false,
            first: i == 0,
            last: i == 1,
            bytes: part,
        }));
    }
}

#[test]
fn a_sync_attach_takes_the_size_then_the_seeds_then_the_repaint_then_the_stream() {
    let (pty, _events, mut script) = connected(StreamMode::Sync);
    let (bytes, seeds) = repaint_of(40, 10, b"hello repaint\r\nsecond line");
    send_repaint(&mut script, 100, wire_size(40, 10), &bytes, seeds);
    script.send(&Message::Out(wire::Out { offset: 100, bytes: b" and after" }));
    wait_for("the stream after the repaint", || pty.screen_text().contains("second line and after"));
    assert!(pty.screen_text().contains("hello repaint"));
    assert_eq!(pty.bottommost_line(), Line(9), "the replica took the repaint's size, 10 rows");
    assert_eq!(
        script.expect("the ack", |g| matches!(g, Got::Ack(_))),
        Got::Ack(110),
        "acked from the repaint"
    );
}

#[test]
fn what_the_host_replays_after_a_repaint_reaches_the_app_as_live_events_do() {
    let (pty, events, mut script) = connected(StreamMode::Sync);
    let (bytes, seeds) = repaint_of(40, 10, b"restored");
    send_repaint(&mut script, 100, wire_size(40, 10), &bytes, seeds);
    // After the repaint's last part, as the host sends them: the replica is the new one by then.
    script.event(100, EventKind::Title("hello"));
    script.event(100, EventKind::Cwd("/"));
    script.event(100, EventKind::ProgramStatus("state=blocked:kind=question:app=claude-code:msg=Which one?"));
    let got = next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_)));
    assert!(matches!(got, PtyEvent::Title(title) if title == "hello"));
    next_event(&events, "folder", |e| matches!(e, PtyEvent::Cwd));
    next_event(&events, "status", |e| matches!(e, PtyEvent::ProgramStatus));
    assert_eq!(pty.cwd().as_deref(), Some(std::path::Path::new("/")));
    let summary = pty.program_status().summary().expect("the report stands");
    assert_eq!(summary.state, crate::program_status::State::Blocked);
    assert_eq!(summary.kind, Some(crate::program_status::Kind::Question));
    wait_for("the repaint", || pty.screen_text().contains("restored"));
}

#[test]
fn too_slow_attaches_again_and_the_repaint_replaces_the_screen() {
    let (pty, _events, mut script) = connected(StreamMode::Raw);
    script.send(&Message::Out(wire::Out { offset: 0, bytes: b"old text\r\n" }));
    wait_for("the raw output", || pty.screen_text().contains("old text"));
    assert_eq!(script.expect("the ack", |g| matches!(g, Got::Ack(_))), Got::Ack(10));
    script.send(&Message::Detached(DetachReason::TooSlow));
    assert_eq!(script.expect("the attach", |g| matches!(g, Got::Attach(_))), Got::Attach(StreamMode::Sync));
    // Still on its way from the stream before: dropped.
    script.send(&Message::Out(wire::Out { offset: 10, bytes: b"stale" }));
    let (bytes, seeds) = repaint_of(80, 24, b"new screen");
    send_repaint(&mut script, 500, wire_size(80, 24), &bytes, seeds);
    script.send(&Message::Out(wire::Out { offset: 500, bytes: b"!" }));
    wait_for("the repaint", || pty.screen_text().contains("new screen!"));
    let text = pty.screen_text();
    assert!(!text.contains("old text") && !text.contains("stale"), "a fresh replica: {text}");
}

#[test]
fn credit_is_acknowledged_after_each_batch_and_at_least_every_64_kib() {
    let (pty, _events, mut script) = connected(StreamMode::Raw);
    let chunk = vec![b'x'; 50 * 1024];
    let mut offset = 0;
    for _ in 0..5 {
        script.send(&Message::Out(wire::Out { offset, bytes: &chunk }));
        offset += chunk.len() as u64;
    }
    let mut acks = Vec::new();
    while acks.last() != Some(&offset) {
        if let Got::Ack(n) = script.expect("an ack", |g| matches!(g, Got::Ack(_))) {
            acks.push(n);
        }
    }
    assert!(acks.windows(2).all(|w| w[0] < w[1]), "acks only grow: {acks:?}");
    assert!(acks.len() <= 5, "at most one per frame here: {acks:?}");
    assert!(ACK_EVERY > chunk.len() as u64);
    drop(pty);
}

#[test]
fn input_goes_in_frames_of_64_kib_with_at_most_256_kib_in_flight() {
    let (pty, _events, mut script) = connected(StreamMode::Raw);
    let total = 600 * 1024;
    pty.write(vec![b'a'; total]);
    let mut got = Vec::new();
    fn take(script: &mut Script, got: &mut Vec<u8>, upto: usize) {
        while got.len() < upto {
            if let Got::Input(bytes) = script.expect("input", |g| matches!(g, Got::Input(_))) {
                assert!(bytes.len() <= wire::MAX_DATA, "a frame of {} bytes", bytes.len());
                got.extend_from_slice(&bytes);
            }
        }
    }
    take(&mut script, &mut got, INPUT_WINDOW as usize);
    assert_eq!(got.len(), INPUT_WINDOW as usize);
    assert!(
        !matches!(script.read(Duration::from_millis(300)), Some(Got::Input(_))),
        "nothing past the window before the host acknowledges"
    );
    script.send(&Message::InputAck(INPUT_WINDOW));
    take(&mut script, &mut got, 2 * INPUT_WINDOW as usize);
    script.send(&Message::InputAck(2 * INPUT_WINDOW));
    take(&mut script, &mut got, total);
    assert_eq!(got.len(), total);
    assert!(got.iter().all(|&b| b == b'a'));
}

#[test]
fn a_resize_waits_for_the_hosts_resize_applied() {
    let (mut pty, _events, mut script) = connected(StreamMode::Raw);
    let wanted = GridSize { cols: 100, rows: 30, ..GRID };
    pty.resize(wanted);
    let asked = script.expect("the resize", |g| matches!(g, Got::Resize(_)));
    assert_eq!(asked, Got::Resize(wire::Size { cols: 100, rows: 30, cell_w: 128, cell_h: 256 }));
    assert_eq!(pty.bottommost_line(), Line(23), "not resized before the host says so");
    script.send(&Message::ResizeApplied(wire::ResizeApplied {
        offset: 0,
        epoch: 1,
        req: 1,
        size: wire_size(100, 30),
    }));
    wait_for("the resize", || pty.bottommost_line() == Line(29));
}

#[test]
fn events_reach_the_app_as_a_local_terminals_do() {
    let (pty, events, mut script) = connected(StreamMode::Raw);
    script.event(0, EventKind::Title("build"));
    let mark = wire::Mark { kind: wire::MarkKind::PromptStart, block: 1, exit_code: None, at_ms: 0 };
    script.event(0, EventKind::Mark(mark));
    script.event(0, EventKind::ClipboardLoad { ticket: 7, selection: b'c' });
    assert!(
        matches!(next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_))), PtyEvent::Title(t) if t == "build")
    );
    let got = next_event(&events, "mark", |e| matches!(e, PtyEvent::Mark(_)));
    assert!(matches!(got, PtyEvent::Mark(Mark::PromptStart(PromptKind::Initial))));
    assert_eq!(pty.at_prompt(), Some(true));
    let PtyEvent::ClipboardLoad(format, reply) =
        next_event(&events, "clipboard load", |e| matches!(e, PtyEvent::ClipboardLoad(..)))
    else {
        unreachable!()
    };
    reply.send(format("clip").into_bytes());
    let answered = script.expect("the clipboard reply", |g| matches!(g, Got::ClipboardReply(..)));
    assert_eq!(answered, Got::ClipboardReply(7, b"clip".to_vec()));
}

#[test]
fn a_connection_that_ends_without_the_programs_exit_is_a_lost_host() {
    let (pty, events, script) = connected(StreamMode::Raw);
    drop(script);
    let got = next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(_)));
    assert!(matches!(got, PtyEvent::Exited(None)));
    assert!(pty.host_lost());
    assert!(!pty.is_alive());
}

#[test]
fn the_programs_exit_then_the_end_is_no_lost_host() {
    let (pty, events, mut script) = connected(StreamMode::Raw);
    script.event(0, EventKind::Exited(wire::ExitInfo { code: 3, signal: 0, at_ms: 0 }));
    let got = next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(_)));
    assert!(matches!(got, PtyEvent::Exited(Some(3))));
    drop(script);
    std::thread::sleep(Duration::from_millis(50));
    assert!(!pty.host_lost());
}

#[test]
fn dropping_a_terminal_closes_its_host_but_not_after_detach() {
    let (pty, _events, mut script) = connected(StreamMode::Raw);
    drop(pty);
    script.expect("the close", |g| *g == Got::Close);
    let (pty, _events, mut script) = connected(StreamMode::Raw);
    assert!(pty.detach());
    drop(pty);
    script.expect("the detach", |g| *g == Got::Detach);
    assert!(
        !matches!(script.read(Duration::from_millis(200)), Some(Got::Close)),
        "a detached terminal leaves its host running"
    );
}

#[test]
fn headless_feeds_no_replica_and_shown_again_repaints() {
    let (pty, events, mut script) = connected(StreamMode::None);
    script.send(&Message::Counters(wire::Counters {
        epoch: 0,
        history_lines: 0,
        trimmed_total: 0,
        screen_rows: 24,
        alt_screen: false,
        revision: 1,
    }));
    script.event(5, EventKind::Title("headless"));
    next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_)));
    pty.set_visible(true);
    assert_eq!(script.expect("the attach", |g| matches!(g, Got::Attach(_))), Got::Attach(StreamMode::Sync));
    let (bytes, seeds) = repaint_of(80, 24, b"shown again");
    send_repaint(&mut script, 900, wire_size(80, 24), &bytes, seeds);
    wait_for("the repaint", || pty.screen_text().contains("shown again"));
    pty.set_visible(false);
    assert_eq!(script.expect("the attach", |g| matches!(g, Got::Attach(_))), Got::Attach(StreamMode::None));
}

#[test]
fn exits_read_as_a_local_terminals() {
    let exit = |code, signal| wire::ExitInfo { code, signal, at_ms: 0 };
    assert_eq!(exit_of(exit(0, 0)), (Some(0), None));
    assert_eq!(exit_of(exit(3, 0)), (Some(3), None));
    assert_eq!(exit_of(exit(0, 9)), (None, Some(9)));
    assert_eq!(exit_of(exit(-1, 0)), (None, None));
}

#[test]
fn readiness_lines_parse() {
    use super::launch::{Ready, parse_line};
    assert_eq!(parse_line("ok 4242"), Ready::Ok(4242));
    assert_eq!(
        parse_line("err could not start /nope: No such file"),
        Ready::Err("could not start /nope: No such file".into())
    );
    assert!(matches!(parse_line("garbage"), Ready::Err(_)));
}
