//! The client against a scripted host on the other end of a socket pair: the attach sequence and
//! the repaint's reset, credit, the input window, attaching again after `TooSlow`, resizes that
//! wait for the host, events, and the end with and without the program's exit. No host process,
//! no PTY.

use std::io;
use std::time::{Duration, Instant};

use wormhole_vt::Terminal;
use wormhole_wire::io::{FrameReader, FrameWriter, RecvError};
use wormhole_wire::{self as wire, CHANNEL_LOCAL, Decoder, DetachReason, EventKind, Message, StreamMode};

use super::events::exit_of;
use super::open::{HostFacts, hosted};
use super::socket::Joined;
use super::{ACK_EVERY, INPUT_WINDOW};
use crate::osc::{Mark, PromptKind};
use crate::sys::socket::Stream;
use crate::types::Line;
use crate::{GridSize, Pty, PtyEvent, PtyEvents, ScreenSnapshot, StartOptions};

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
    reader: FrameReader<Stream>,
    writer: FrameWriter<Stream>,
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
    connected_to_minor(first, wire::version::PROTO_MINOR)
}

/// [`connected`] to a host whose `Welcome` said protocol minor `minor`.
fn connected_to_minor(first: StreamMode, minor: u16) -> (Pty, PtyEvents, Script) {
    let (ours, theirs) = Stream::pair().expect("a socket pair");
    let joined = Joined { stream: ours, dec: Decoder::new() };
    let options = StartOptions { scrollback_lines: 1000, ..StartOptions::default() };
    let (pty, events) = hosted(
        joined,
        "abcdef123456",
        GRID,
        GRID,
        &options,
        first,
        HostFacts { pid: std::process::id(), minor, features: wire::Features::NONE, replay: None },
    )
    .expect("a client");
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
fn repaint_of(cols: usize, rows: usize, text: &[u8]) -> (Vec<u8>, wormhole_vt::repaint::Seeds) {
    let mut term = Terminal::builder().size(cols, rows).scrollback(1000).build();
    term.feed(text);
    let mut bytes = Vec::new();
    let made = term.repaint(&mut bytes).expect("between sequences");
    (bytes, made.seeds)
}

/// Send a repaint at `offset` as the host does: the size, the seeds, the parts.
fn send_repaint(
    script: &mut Script,
    offset: u64,
    size: wire::Size,
    bytes: &[u8],
    seeds: wormhole_vt::repaint::Seeds,
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
fn the_replica_wakes_the_app_for_a_synchronized_update_when_it_begins_and_ends_and_not_between() {
    // The update's real timeout (150 ms) gives up on it: on a slow runner it can pass before the
    // check of the quiet, and that run proves nothing. It is run again, a few times.
    let margin = Duration::from_millis(50);
    for _ in 0..5 {
        let (pty, events, mut script) = connected(StreamMode::Raw);
        let mut offset = 0;
        let mut send = |script: &mut Script, bytes: &[u8]| {
            script.send(&Message::Out(wire::Out { offset, bytes }));
            offset += bytes.len() as u64;
        };
        let began = Instant::now();
        send(&mut script, b"\x1b[?2026h");
        next_event(&events, "the update's begin", |e| matches!(e, PtyEvent::Output));
        send(&mut script, b"inside");
        wait_for("the update's text", || pty.screen_text().contains("inside"));
        if began.elapsed() + margin >= crate::screen::SYNC_TIMEOUT {
            drop(pty);
            continue;
        }
        assert_eq!(events.queued(), 0, "nothing to draw while the update holds the screen");
        send(&mut script, b"\x1b[?2026l");
        next_event(&events, "the update's end", |e| matches!(e, PtyEvent::Output));
        drop(pty);
        return;
    }
    panic!("the update timed out before it could be checked, five times in a row");
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
    let got = next_event(&events, "mark", |e| matches!(e, PtyEvent::Mark(..)));
    assert!(matches!(got, PtyEvent::Mark(Mark::PromptStart(PromptKind::Initial), _)));
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
fn a_lost_host_inside_a_synchronized_update_leaves_the_last_screen_shown() {
    let (pty, events, mut script) = connected(StreamMode::Raw);
    // The UI keeps its copy of the frame from before an update until it ends.
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot);
    script.send(&Message::Out(wire::Out { offset: 0, bytes: b"\x1b[?2026hlast words" }));
    wait_for("the update's text", || pty.screen_text().contains("last words"));
    drop(script);
    next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(None)));
    // At once: the update's own timeout (150 ms) would show it too, but nothing runs for it now.
    pty.snapshot(&mut snapshot);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 'l', "the screen is shown, not the frame from before");
}

#[test]
fn the_end_after_the_programs_exit_inside_a_synchronized_update_leaves_the_last_screen_shown() {
    let (pty, events, mut script) = connected(StreamMode::Raw);
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot);
    script.send(&Message::Out(wire::Out { offset: 0, bytes: b"\x1b[?2026hlast words" }));
    wait_for("the update's text", || pty.screen_text().contains("last words"));
    // The exit is heard first, then the connection ends: the update is still held.
    script.event(0, EventKind::Exited(wire::ExitInfo { code: 0, signal: 0, at_ms: 0 }));
    next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(Some(0))));
    drop(script);
    // Letting go of the held update wakes the proxy: that output event is the release.
    next_event(&events, "release", |e| matches!(e, PtyEvent::Output));
    pty.snapshot(&mut snapshot);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 'l', "the screen is shown, not the frame from before");
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

/// A host that takes a connection and never answers its pings is called not responding after the
/// patience, and an answer (anything it says) clears it. Real time: the thread's own clock.
#[test]
fn a_host_that_never_answers_a_ping_is_not_responding_until_it_speaks() {
    let (pty, _events, mut script) = connected(StreamMode::Sync);
    assert_eq!(pty.link_stats().expect("hosted").not_responding_for, None);
    // It reads its pings and says nothing.
    let until = Instant::now() + Duration::from_secs(15);
    while pty.link_stats().and_then(|l| l.not_responding_for).is_none() {
        assert!(Instant::now() < until, "the host was never called not responding");
        let _ = script.read(Duration::from_millis(100));
    }
    // Any frame from it, a Pong or not, says it is there.
    script.send(&Message::Pong { n: 1, pty_progress: 0 });
    wait_for("the host to be responsive again", || {
        pty.link_stats().is_some_and(|l| l.not_responding_for.is_none())
    });
}

/// A host that said protocol minor 1 in its `Welcome` skips `ClearHistory`, and the terminal knows.
#[test]
fn a_minor_1_host_is_known_not_to_forget_history() {
    let (pty, _events, _script) = connected(StreamMode::Sync);
    assert_eq!(pty.host_forgets_history(), Some(true), "a host of this build's minor does");
    let (old, _events, _script) = connected_to_minor(StreamMode::Sync, 1);
    assert_eq!(old.host_minor(), Some(1));
    assert_eq!(old.host_forgets_history(), Some(false));
}

/// What `hello` makes of each thing a host can answer a `Hello` with (or not answer).
mod handshake {
    use std::io::Write as _;

    use super::*;
    use crate::client::AttachError;
    use crate::client::socket::hello;

    /// Run `hello` against a host that does `answer` with its end of the pair, then hangs up.
    fn hello_against(answer: impl FnOnce(&Stream) + Send + 'static) -> Result<wire::Welcome, AttachError> {
        let (ours, theirs) = Stream::pair().expect("a socket pair");
        let host = std::thread::spawn(move || {
            // Read the Hello first, as a host does.
            let mut dec = Decoder::new();
            let mut buf = [0u8; 512];
            while dec.next_frame().map(|f| f.is_none()).unwrap_or(true) {
                let n = std::io::Read::read(&mut &theirs, &mut buf).unwrap_or(0);
                if n == 0 {
                    return;
                }
                dec.push(&buf[..n]);
            }
            answer(&theirs);
        });
        let mut dec = Decoder::new();
        let result = hello(&ours, &mut dec, "abcdefghij12", Instant::now() + PATIENCE);
        host.join().expect("the host thread");
        result
    }

    fn frame(msg: &Message<'_>) -> Vec<u8> {
        let mut bytes = Vec::new();
        wire::encode(&mut bytes, CHANNEL_LOCAL, 0, msg).expect("encodes");
        bytes
    }

    fn error(code: u16) -> Vec<u8> {
        frame(&Message::Error(wire::ErrorMsg { ref_type: 0, code, text: "because" }))
    }

    #[test]
    fn a_host_that_refuses_the_peer_says_so_and_is_refused() {
        let result = hello_against(|s| {
            let _ = (&mut &*s).write_all(&error(wire::error_code::PEER_REFUSED));
        });
        assert_eq!(result.unwrap_err(), AttachError::Refused { said: true, why: "because".into() });
    }

    #[test]
    fn a_host_older_than_protocol_1_3_closes_without_a_word_and_is_refused_too() {
        // It took the connection and hung up: it is there, which is not "gone".
        let result = hello_against(|_| {});
        assert!(matches!(result, Err(AttachError::Refused { said: false, .. })), "{result:?}");
    }

    #[test]
    fn a_busy_host_is_busy() {
        let result = hello_against(|s| {
            let _ = (&mut &*s).write_all(&error(wire::error_code::BUSY));
        });
        assert_eq!(result.unwrap_err(), AttachError::Busy);
    }

    #[test]
    fn a_host_of_another_protocol_is_incompatible() {
        let result = hello_against(|s| {
            let inc = wire::Incompatible {
                host_min: 7,
                host_max: 8,
                host_version: wire::AppVersion { major: 9, minor: 0, patch: 1 },
            };
            let _ = (&mut &*s).write_all(&frame(&Message::Incompatible(inc)));
        });
        let Err(AttachError::Incompatible { host_min, host_max, host_version, .. }) = result else {
            panic!("incompatible: {result:?}")
        };
        assert_eq!((host_min, host_max, host_version.as_str()), (7, 8, "9.0.1"));
    }

    #[test]
    fn a_socket_that_is_another_hosts_is_not_this_host() {
        let result = hello_against(|s| {
            let mut id = [0u8; 12];
            id.copy_from_slice(b"zzzzzzzzzzzz");
            let welcome = wire::Welcome {
                proto: 1,
                proto_minor: 3,
                state: wire::HostState::Running,
                host_version: wire::AppVersion::default(),
                pid: 1,
                session_id: id,
                features: wire::Features::NONE,
                started_at_ms: 0,
                first_block_id: 0,
                size: wire::Size { cols: 80, rows: 24, cell_w: 8, cell_h: 16 },
                counters: wire::Counters {
                    epoch: 0,
                    history_lines: 0,
                    trimmed_total: 0,
                    screen_rows: 24,
                    alt_screen: false,
                    revision: 0,
                },
                exit: None,
                token: None,
            };
            let _ = (&mut &*s).write_all(&frame(&Message::Welcome(welcome)));
        });
        assert!(matches!(result, Err(AttachError::Unreachable(_))), "{result:?}");
    }
}

#[test]
fn the_replica_draws_the_seed_only_as_a_host_that_has_it() {
    use super::open::seed_drawn_by;
    assert_eq!(seed_drawn_by(Some("old"), 4), Some("old"));
    assert_eq!(seed_drawn_by(Some("old"), 9), Some("old"));
    assert_eq!(
        seed_drawn_by(Some("old"), 3),
        None,
        "a host that skipped the seed: the replica must not draw it"
    );
    assert_eq!(seed_drawn_by(None, 4), None);
}
