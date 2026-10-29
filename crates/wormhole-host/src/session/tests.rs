//! The session over a loopback: frames encoded as a client would, the session's output decoded as
//! a client would, a scripted terminal, and a virtual clock (every call takes the time).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_term::hosted::{Cursor, Gaps, HostEvent, NotReady, Repainted, ResizeMark, Stream, Wake};
use wormhole_wire::{
    self as wire, AppVersion, Attach, CHANNEL_LOCAL, Decoder, DetachReason, EventKind, Features, HostState,
    Message, Role, StreamMode, error_code, ty,
};

use super::{Config, ConnId, Notice, Session};
use crate::program::{Program, Spawner};

const SIZE: wire::Size = wire::Size { cols: 80, rows: 24, cell_w: 128, cell_h: 256 };

/// What the scripted terminal was told, and what it says.
#[derive(Default)]
struct State {
    input: Vec<u8>,
    acked: u64,
    resizes: Vec<(u32, wire::Size)>,
    closed: Option<Duration>,
    finished: bool,
    exit: Option<wire::ExitInfo>,
    ready: bool,
    controller: bool,
    progress: u64,
    clipboard: Vec<(u32, Vec<u8>)>,
    /// What the terminal says it remembers (title, folder, reports).
    remembered: Vec<HostEvent>,
}

struct Fake {
    stream: Arc<Stream>,
    state: Rc<RefCell<State>>,
    size: wire::Size,
    epoch: u32,
}

impl Program for Fake {
    fn pid(&self) -> u32 {
        4242
    }
    fn size(&self) -> wire::Size {
        self.size
    }
    fn input(&mut self, bytes: &[u8]) {
        self.state.borrow_mut().input.extend_from_slice(bytes);
    }
    fn input_acked(&self) -> u64 {
        self.state.borrow().acked
    }
    fn resize(&mut self, req: u32, size: wire::Size) {
        self.state.borrow_mut().resizes.push((req, size));
        self.size = size;
        self.epoch += 1;
        self.stream.push_mark(ResizeMark { offset: self.stream.total(), epoch: self.epoch, req, size });
    }
    fn follow(&self, bytes: bool, gap: Duration, wake: Wake) -> Cursor {
        self.stream.register(bytes, gap, wake)
    }
    fn repaint(&self, max_history: usize, out: &mut Vec<u8>, wake: Wake) -> Result<Repainted, NotReady> {
        if !self.state.borrow().ready {
            return Err(NotReady::Sequence);
        }
        out.extend_from_slice(b"\x1bc");
        out.extend_from_slice(format!("history {max_history}").as_bytes());
        // A big screen, so it goes in parts.
        out.resize(out.len() + 100_000, b'x');
        let offset = self.stream.total();
        Ok(Repainted {
            cursor: self.stream.register(true, Duration::ZERO, wake),
            offset,
            epoch: self.epoch,
            size: self.size,
            block: 3,
            link_seq: 4,
            image_handle: 5,
            gaps: Gaps::empty(),
            history_lines: 0,
        })
    }
    fn remembered(&self) -> Vec<HostEvent> {
        self.state.borrow().remembered.clone()
    }
    fn counters(&self) -> wire::Counters {
        wire::Counters {
            epoch: self.epoch,
            history_lines: 0,
            trimmed_total: 0,
            screen_rows: self.size.rows,
            alt_screen: false,
            revision: self.stream.total(),
        }
    }
    fn progress(&self) -> u64 {
        self.state.borrow().progress
    }
    fn exit(&self) -> Option<wire::ExitInfo> {
        self.state.borrow().exit
    }
    fn finished(&self) -> bool {
        self.state.borrow().finished
    }
    fn close(&mut self, grace: Duration) {
        self.state.borrow_mut().closed = Some(grace);
    }
    fn set_colors(&mut self, _: &wire::Colors<'_>) {}
    fn set_color_scheme(&mut self, _: wire::ColorScheme) {}
    fn set_options(&mut self, _: &wire::Options) {}
    fn clear_history(&mut self) {}
    fn set_controller(&self, attached: bool) {
        self.state.borrow_mut().controller = attached;
    }
    fn clipboard_reply(&self, ticket: u32, data: &[u8]) {
        self.state.borrow_mut().clipboard.push((ticket, data.to_vec()));
    }
    fn clipboard_expire(&self, _: Instant) -> Option<Instant> {
        None
    }
    fn first_block_id(&self) -> u64 {
        1
    }
}

struct FakeSpawner {
    stream: Arc<Stream>,
    state: Rc<RefCell<State>>,
}

impl Spawner for FakeSpawner {
    type Program = Fake;

    fn spawn(&mut self, spawn: &wire::Spawn<'_>, wake: Wake) -> anyhow::Result<(Fake, Cursor)> {
        if spawn.program == "/no/such/program" {
            anyhow::bail!("could not start /no/such/program: No such file or directory");
        }
        let cursor = self.stream.register(true, Duration::ZERO, wake);
        let fake =
            Fake { stream: self.stream.clone(), state: self.state.clone(), size: spawn.size, epoch: 0 };
        Ok((fake, cursor))
    }
}

/// A frame as a test reads it.
#[derive(Clone, Debug, PartialEq)]
enum Got {
    Welcome(wire::Welcome),
    Info(wire::Welcome),
    Incompatible,
    Error(u8, u16),
    Detached(DetachReason),
    Pong(u64, u64),
    Out(u64, Vec<u8>),
    Repaint { offset: u64, first: bool, last: bool, len: usize },
    Resized(wire::ResizeApplied),
    Event(u64, String),
    InputAck(u64),
    Counters(wire::Counters),
    Other(u8),
}

fn got(msg: &Message<'_>) -> Got {
    match msg {
        Message::Welcome(w) => Got::Welcome(*w),
        Message::Info(w) => Got::Info(*w),
        Message::Incompatible(_) => Got::Incompatible,
        Message::Error(e) => Got::Error(e.ref_type, e.code),
        Message::Detached(r) => Got::Detached(*r),
        Message::Pong { n, pty_progress } => Got::Pong(*n, *pty_progress),
        Message::Out(o) => Got::Out(o.offset, o.bytes.to_vec()),
        Message::Repaint(r) => {
            Got::Repaint { offset: r.offset, first: r.first, last: r.last, len: r.bytes.len() }
        }
        Message::ResizeApplied(r) => Got::Resized(*r),
        Message::Event(e) => Got::Event(e.offset, format!("{:?}", e.kind)),
        Message::InputAck(n) => Got::InputAck(*n),
        Message::Counters(c) => Got::Counters(*c),
        other => Got::Other(other.ty()),
    }
}

struct Rig {
    session: Session<FakeSpawner>,
    stream: Arc<Stream>,
    state: Rc<RefCell<State>>,
    t0: Instant,
    now: Instant,
}

impl Rig {
    fn new() -> Self {
        Self::with(|_| {})
    }

    fn with(tune: impl FnOnce(&mut Config)) -> Self {
        let mut config = Config::new("abcdefghij12".into(), AppVersion { major: 1, minor: 2, patch: 3 }, 77);
        tune(&mut config);
        let stream = Arc::new(Stream::default());
        let state = Rc::new(RefCell::new(State { ready: true, ..State::default() }));
        let spawner = FakeSpawner { stream: stream.clone(), state: state.clone() };
        let t0 = Instant::now();
        let wake: Wake = Arc::new(|| {});
        Self { session: Session::new(config, spawner, wake, t0), stream, state, t0, now: t0 }
    }

    fn advance(&mut self, by: Duration) {
        self.now += by;
    }

    fn connect(&mut self) -> ConnId {
        self.session.accept(self.now).expect("accepted")
    }

    fn send(&mut self, id: ConnId, msg: &Message<'_>) {
        let mut bytes = Vec::new();
        wire::encode(&mut bytes, CHANNEL_LOCAL, 0, msg).expect("encodes");
        self.session.received(id, &bytes, self.now);
    }

    /// Pump, and take everything queued for `id` as the client would read it.
    fn recv(&mut self, id: ConnId) -> Vec<Got> {
        self.session.pump(self.now);
        let out = self.session.output(id).to_vec();
        self.session.wrote(id, out.len());
        let mut dec = Decoder::new();
        dec.push(&out);
        let mut all = Vec::new();
        while let Some(frame) = dec.next_frame().expect("the host's frames decode") {
            all.push(got(&frame.message));
        }
        all
    }

    fn hello(&mut self, id: ConnId, role: Role, force: bool) -> Vec<Got> {
        let hello = wire::Hello {
            proto_min: 1,
            proto_max: 1,
            proto_minor: 0,
            role,
            force,
            features: Features::NONE,
            client_version: AppVersion::default(),
            client_name: "test",
            ticket: None,
        };
        self.send(id, &Message::Hello(hello));
        self.recv(id)
    }

    fn spawn_program(&mut self, id: ConnId, program: &str) -> Vec<Got> {
        let spawn = wire::Spawn {
            size: SIZE,
            options: wire::Options { scrollback_lines: 1000, flags: 0 },
            program,
            cwd: "/",
            args: wire::List::Owned(vec![]),
            env: wire::List::Owned(vec![("TERM", "xterm-256color")]),
            colors: None,
        };
        self.send(id, &Message::Spawn(spawn));
        self.recv(id)
    }

    /// A controller that spawned the program.
    fn spawned(&mut self) -> ConnId {
        let id = self.connect();
        self.hello(id, Role::Controller, false);
        let got = self.spawn_program(id, "/bin/sh");
        assert!(matches!(got.as_slice(), [Got::Info(w)] if w.pid == 4242), "{got:?}");
        id
    }

    fn attach(&mut self, id: ConnId, stream: StreamMode, window: u32) -> Vec<Got> {
        self.send(id, &Message::Attach(Attach { stream, history_lines: 500, window, resume_from: None }));
        self.recv(id)
    }
}

fn outs(got: &[Got]) -> Vec<u8> {
    got.iter().filter_map(|g| if let Got::Out(_, b) = g { Some(b.clone()) } else { None }).flatten().collect()
}

#[test]
fn hello_welcome_spawn_info_and_the_ready_notice() {
    let mut rig = Rig::new();
    let id = rig.connect();
    let got = rig.hello(id, Role::Controller, false);
    let [Got::Welcome(welcome)] = got.as_slice() else { panic!("{got:?}") };
    assert_eq!((welcome.pid, welcome.state, &welcome.session_id), (0, HostState::Running, b"abcdefghij12"));
    assert_eq!(welcome.host_version, AppVersion { major: 1, minor: 2, patch: 3 });
    let got = rig.spawn_program(id, "/bin/sh");
    assert!(matches!(got.as_slice(), [Got::Info(w)] if w.pid == 4242 && w.size == SIZE), "{got:?}");
    assert_eq!(rig.session.take_notices(), [Notice::Spawned { pid: 4242 }]);
    let got = rig.spawn_program(id, "/bin/sh");
    assert_eq!(got, [Got::Error(ty::SPAWN, error_code::BAD_STATE)], "one program per host");
}

#[test]
fn a_spawn_that_fails_says_so_and_ends_the_host() {
    let mut rig = Rig::new();
    let id = rig.connect();
    rig.hello(id, Role::Controller, false);
    let got = rig.spawn_program(id, "/no/such/program");
    assert_eq!(got[0], Got::Error(ty::SPAWN, error_code::SPAWN_FAILED));
    assert!(
        matches!(rig.session.take_notices().as_slice(), [Notice::SpawnFailed(text)] if text.contains("No such"))
    );
    assert!(rig.session.is_done());
}

#[test]
fn nobody_spawning_in_time_ends_the_host() {
    let mut rig = Rig::new();
    rig.advance(Duration::from_secs(9));
    rig.session.pump(rig.now);
    assert!(!rig.session.is_done());
    rig.advance(Duration::from_secs(2));
    rig.session.pump(rig.now);
    assert!(rig.session.is_done());
    assert!(matches!(rig.session.take_notices().as_slice(), [Notice::SpawnFailed(_)]));
}

#[test]
fn the_spawner_reads_from_the_first_byte_and_others_from_now() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.stream.append(b"$ prompt");
    let got = rig.attach(id, StreamMode::Raw, 0);
    assert_eq!(got.first(), Some(&Got::Out(0, b"$ prompt".to_vec())), "{got:?}");
    let other = rig.connect();
    rig.hello(other, Role::Observer, false);
    rig.attach(other, StreamMode::Raw, 0);
    rig.stream.append(b" more");
    let got = rig.recv(other);
    assert_eq!(got.first(), Some(&Got::Out(8, b" more".to_vec())), "{got:?}");
}

#[test]
fn resizes_and_events_arrive_between_exactly_their_bytes() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 0);
    rig.stream.append(b"before");
    rig.send(id, &Message::Resize(wire::Resize { req: 9, size: wire::Size { cols: 100, ..SIZE } }));
    rig.session.pump(rig.now);
    rig.stream.push_event(rig.stream.total() + 5, HostEvent::Bell);
    rig.stream.append(b"after");
    let got: Vec<Got> = rig.recv(id).into_iter().filter(|g| !matches!(g, Got::Counters(_))).collect();
    assert_eq!(got[0], Got::Out(0, b"before".to_vec()), "{got:?}");
    assert!(
        matches!(got[1], Got::Resized(r) if r.req == 9 && r.offset == 6 && r.size.cols == 100),
        "{got:?}"
    );
    assert_eq!(got[2], Got::Out(6, b"after".to_vec()));
    assert_eq!(got[3], Got::Event(11, "Bell".into()));
}

#[test]
fn resizes_are_coalesced_to_the_latest_per_turn_window() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.send(id, &Message::Resize(wire::Resize { req: 1, size: wire::Size { cols: 90, ..SIZE } }));
    rig.session.pump(rig.now);
    rig.advance(Duration::from_millis(2));
    rig.send(id, &Message::Resize(wire::Resize { req: 2, size: wire::Size { cols: 91, ..SIZE } }));
    rig.send(id, &Message::Resize(wire::Resize { req: 3, size: wire::Size { cols: 92, ..SIZE } }));
    let pump = rig.session.pump(rig.now);
    assert_eq!(rig.state.borrow().resizes.len(), 1, "the second is held back");
    assert_eq!(pump.next, Some(rig.t0 + Duration::from_millis(8)));
    rig.advance(Duration::from_millis(6));
    rig.session.pump(rig.now);
    let resizes = rig.state.borrow().resizes.clone();
    assert_eq!(resizes.iter().map(|(req, _)| *req).collect::<Vec<_>>(), [1, 3], "the latest wins");
    rig.send(id, &Message::Resize(wire::Resize { req: 4, size: wire::Size { cols: 0, ..SIZE } }));
    assert_eq!(rig.recv(id).last(), Some(&Got::Error(ty::RESIZE, error_code::INVALID)));
}

#[test]
fn output_stops_at_the_window_and_an_ack_opens_it() {
    let mut rig = Rig::with(|c| c.min_window = 1);
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 10);
    rig.stream.append(b"0123456789abcdef");
    let got = rig.recv(id);
    assert_eq!(outs(&got), b"0123456789");
    rig.send(id, &Message::Ack(1_000));
    let got = rig.recv(id);
    assert_eq!(outs(&got), b"abcdef", "an ack past what was sent counts as all of it");
    rig.stream.append(b"ghijklmnopqrstuvwxyz");
    assert_eq!(outs(&rig.recv(id)), b"ghij", "the window is from the last byte sent (10 + 10 = 20)");
}

#[test]
fn a_stalled_client_is_cut_loose_after_a_quarter_second_and_can_attach_again() {
    let mut rig = Rig::with(|c| c.min_window = 1);
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 4);
    rig.stream.append(b"12345678");
    rig.recv(id);
    rig.advance(Duration::from_millis(200));
    assert!(!rig.recv(id).contains(&Got::Detached(DetachReason::TooSlow)));
    rig.advance(Duration::from_millis(60));
    let got = rig.recv(id);
    assert!(got.contains(&Got::Detached(DetachReason::TooSlow)), "{got:?}");
    rig.stream.append(b"more");
    assert!(outs(&rig.recv(id)).is_empty(), "nothing more is streamed to it");
    let got = rig.attach(id, StreamMode::Sync, 0);
    assert!(
        got.iter().any(|g| matches!(g, Got::Repaint { first: true, .. })),
        "it comes back with a repaint"
    );
}

#[test]
fn a_sync_attach_is_the_size_the_seeds_the_repaint_in_parts_then_the_bytes() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.stream.append(b"earlier output");
    let got = rig.attach(id, StreamMode::Sync, 0);
    let Got::Resized(size) = got[0] else { panic!("{got:?}") };
    assert_eq!((size.offset, size.size), (14, SIZE));
    assert!(matches!(&got[1], Got::Event(14, seeds) if seeds.contains("RepaintSeeds")), "{got:?}");
    let parts: Vec<_> = got.iter().filter(|g| matches!(g, Got::Repaint { .. })).collect();
    assert_eq!(parts.len(), 2, "100 KB goes in two parts of at most 64 KiB");
    assert!(matches!(parts[0], Got::Repaint { offset: 14, first: true, last: false, len: 65536 }));
    assert!(matches!(parts[1], Got::Repaint { offset: 14, first: false, last: true, .. }));
    rig.stream.append(b"new");
    assert_eq!(rig.recv(id).first(), Some(&Got::Out(14, b"new".to_vec())));
}

/// The title, the folder and one report, as a terminal that heard them remembers them.
fn remember_some(rig: &Rig) {
    rig.state.borrow_mut().remembered = vec![
        HostEvent::Title("build".into()),
        HostEvent::Cwd("/work".into()),
        HostEvent::ProgramStatus("state=working".into()),
    ];
}

/// The remembered events among `got`, in the order they came, with the index each came at.
fn remembered_in(got: &[Got]) -> Vec<(usize, Got)> {
    got.iter()
        .cloned()
        .enumerate()
        .filter(|(_, g)| {
            matches!(g, Got::Event(_, e) if e.starts_with("Title(") || e.starts_with("Cwd(") || e.starts_with("ProgramStatus("))
        })
        .collect()
}

fn the_three(offset: u64) -> Vec<Got> {
    vec![
        Got::Event(offset, "Title(\"build\")".into()),
        Got::Event(offset, "Cwd(\"/work\")".into()),
        Got::Event(offset, "ProgramStatus(\"state=working\")".into()),
    ]
}

#[test]
fn every_attach_is_told_the_title_the_folder_and_the_report_in_order() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.stream.append(b"earlier output");
    remember_some(&rig);

    // Raw, from the stream's first byte: at once, before the bytes.
    let got = rig.attach(id, StreamMode::Raw, 0);
    let told = remembered_in(&got);
    assert_eq!(told.iter().map(|(_, g)| g.clone()).collect::<Vec<_>>(), the_three(0), "{got:?}");
    assert!(
        got.iter().position(|g| matches!(g, Got::Out(..))) > Some(told[2].0),
        "before the bytes: {got:?}"
    );

    // Headless, from now: at once, at the stream's offset.
    let other = rig.connect();
    rig.hello(other, Role::Observer, false);
    let got = rig.attach(other, StreamMode::None, 0);
    assert_eq!(got.iter().filter(|g| matches!(g, Got::Event(..))).cloned().collect::<Vec<_>>(), the_three(0));

    // Raw from now.
    let third = rig.connect();
    rig.hello(third, Role::Observer, false);
    let got = rig.attach(third, StreamMode::Raw, 0);
    assert_eq!(
        got.iter().filter(|g| matches!(g, Got::Event(..))).cloned().collect::<Vec<_>>(),
        the_three(14)
    );

    // Sync: after the seeds and the repaint's last part, at the repaint's offset.
    let got = rig.attach(id, StreamMode::Sync, 0);
    let told = remembered_in(&got);
    assert_eq!(told.iter().map(|(_, g)| g.clone()).collect::<Vec<_>>(), the_three(14), "{got:?}");
    let seeds = got.iter().position(|g| matches!(g, Got::Event(_, e) if e.contains("RepaintSeeds")));
    let last = got.iter().position(|g| matches!(g, Got::Repaint { last: true, .. }));
    assert!(seeds < last && last < Some(told[0].0), "seeds, the repaint, then the three: {got:?}");
    assert_eq!(told.windows(2).filter(|w| w[1].0 != w[0].0 + 1).count(), 0, "together: {got:?}");
}

#[test]
fn an_attach_with_nothing_remembered_sends_no_such_events() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    let got = rig.attach(id, StreamMode::None, 0);
    assert!(remembered_in(&got).is_empty(), "{got:?}");
}

#[test]
fn a_sync_attach_waiting_for_the_parser_tells_them_once_the_repaint_is_out() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    remember_some(&rig);
    rig.state.borrow_mut().ready = false;
    let got = rig.attach(id, StreamMode::Sync, 0);
    assert!(remembered_in(&got).is_empty(), "{got:?}");
    rig.state.borrow_mut().ready = true;
    rig.advance(Duration::from_millis(5));
    let got = rig.recv(id);
    assert_eq!(remembered_in(&got).len(), 3, "{got:?}");
}

#[test]
fn a_sync_attach_waits_for_the_parser_to_be_between_sequences() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.state.borrow_mut().ready = false;
    let got = rig.attach(id, StreamMode::Sync, 0);
    assert!(!got.iter().any(|g| matches!(g, Got::Repaint { .. })), "{got:?}");
    let pump = rig.session.pump(rig.now);
    assert!(pump.next.is_some_and(|at| at <= rig.now + Duration::from_millis(5)), "it tries again soon");
    rig.state.borrow_mut().ready = true;
    rig.advance(Duration::from_millis(5));
    assert!(rig.recv(id).iter().any(|g| matches!(g, Got::Repaint { last: true, .. })));
}

#[test]
fn a_headless_attach_gets_events_and_counters_but_no_bytes() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.attach(id, StreamMode::None, 0);
    rig.stream.push_event(3, HostEvent::Title("build".into()));
    rig.stream.append(b"abc");
    let got = rig.recv(id);
    assert!(outs(&got).is_empty());
    assert!(got.contains(&Got::Event(3, "Title(\"build\")".into())), "{got:?}");
    // The attach sent the first counters: the next wait out the 16 ms.
    rig.advance(Duration::from_millis(16));
    let got = rig.recv(id);
    assert!(outs(&got).is_empty());
    assert!(got.iter().any(|g| matches!(g, Got::Counters(c) if c.revision == 3)), "{got:?}");
    rig.stream.append(b"d");
    assert!(!rig.recv(id).iter().any(|g| matches!(g, Got::Counters(_))), "at most every 16 ms");
    rig.advance(Duration::from_millis(16));
    assert!(rig.recv(id).iter().any(|g| matches!(g, Got::Counters(c) if c.revision == 4)));
}

#[test]
fn the_clipboard_is_the_controllers_alone() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 0);
    assert!(rig.state.borrow().controller, "an attached controller answers the clipboard");
    let observer = rig.connect();
    rig.hello(observer, Role::Observer, false);
    rig.attach(observer, StreamMode::Raw, 0);
    rig.stream.push_event(0, HostEvent::ClipboardLoad { ticket: 7, selection: b'c' });
    assert!(rig.recv(id).iter().any(|g| matches!(g, Got::Event(_, e) if e.contains("ClipboardLoad"))));
    assert!(!rig.recv(observer).iter().any(|g| matches!(g, Got::Event(..))));
    rig.send(id, &Message::ClipboardReply(wire::ClipboardReply { ticket: 7, data: b"hi" }));
    assert_eq!(rig.state.borrow().clipboard, [(7, b"hi".to_vec())]);
    rig.session.closed(id);
    assert!(!rig.state.borrow().controller, "gone: the tickets out are answered empty");
}

#[test]
fn input_goes_to_the_program_and_its_ack_comes_back() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.send(id, &Message::Input(b"ls\r"));
    assert_eq!(rig.state.borrow().input, b"ls\r");
    rig.state.borrow_mut().acked = 3;
    assert!(rig.recv(id).contains(&Got::InputAck(3)));
    assert!(!rig.recv(id).contains(&Got::InputAck(3)), "once");
}

#[test]
fn roles_limit_what_a_connection_may_do() {
    let mut rig = Rig::new();
    let _controller = rig.spawned();
    let observer = rig.connect();
    rig.hello(observer, Role::Observer, false);
    rig.send(observer, &Message::Input(b"x"));
    assert_eq!(rig.recv(observer), [Got::Error(ty::INPUT, error_code::NOT_CONTROLLER)]);
    let admin = rig.connect();
    rig.hello(admin, Role::Admin, false);
    rig.send(
        admin,
        &Message::Attach(Attach { stream: StreamMode::Raw, history_lines: 0, window: 0, resume_from: None }),
    );
    assert_eq!(rig.recv(admin), [Got::Error(ty::ATTACH, error_code::NOT_CONTROLLER)]);
    rig.send(admin, &Message::GetInfo);
    assert!(matches!(rig.recv(admin).as_slice(), [Got::Info(w)] if w.pid == 4242));
    rig.state.borrow_mut().progress = 77;
    rig.send(observer, &Message::Ping(5));
    assert_eq!(rig.recv(observer), [Got::Pong(5, 77)]);
}

#[test]
fn a_second_controller_waits_unless_it_takes_over() {
    let mut rig = Rig::new();
    let first = rig.spawned();
    let second = rig.connect();
    assert_eq!(rig.hello(second, Role::Controller, false), [Got::Error(ty::HELLO, error_code::BUSY)]);
    assert!(rig.session.should_close(second));
    let third = rig.connect();
    assert!(matches!(rig.hello(third, Role::Controller, true).as_slice(), [Got::Welcome(_)]));
    assert_eq!(rig.recv(first), [Got::Detached(DetachReason::TakenOver)]);
    assert!(rig.session.should_close(first));
}

#[test]
fn the_handshake_is_hello_in_time_and_in_range() {
    let mut rig = Rig::new();
    let late = rig.connect();
    rig.advance(Duration::from_millis(2100));
    rig.session.pump(rig.now);
    assert!(rig.session.should_close(late), "no Hello within 2 s");
    let rude = rig.connect();
    rig.send(rude, &Message::Ping(1));
    assert_eq!(rig.recv(rude), [Got::Error(ty::PING, error_code::BAD_STATE)]);
    assert!(rig.session.should_close(rude));
    let future = rig.connect();
    let hello = wire::Hello {
        proto_min: 7,
        proto_max: 9,
        proto_minor: 0,
        role: Role::Controller,
        force: false,
        features: Features::NONE,
        client_version: AppVersion::default(),
        client_name: "",
        ticket: None,
    };
    rig.send(future, &Message::Hello(hello));
    assert_eq!(rig.recv(future), [Got::Incompatible]);
    let garbage = rig.connect();
    rig.session.received(garbage, &[0xff; 16], rig.now);
    assert!(rig.session.should_close(garbage), "a frame too large for the handshake");
}

#[test]
fn at_most_four_connections() {
    let mut rig = Rig::new();
    for _ in 0..4 {
        rig.connect();
    }
    let refusal = rig.session.accept(rig.now).expect_err("the fifth is refused");
    let mut dec = Decoder::new();
    dec.push(&refusal);
    let frame = dec.next_frame().expect("decodes").expect("a frame");
    assert!(matches!(frame.message, Message::Error(e) if e.code == error_code::BUSY));
}

#[test]
fn close_hangs_up_and_the_host_ends_when_that_is_over() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.send(id, &Message::Close { grace_ms: 500 });
    assert_eq!(rig.state.borrow().closed, Some(Duration::from_millis(500)));
    rig.session.pump(rig.now);
    assert!(!rig.session.is_done(), "until the hangup is over");
    rig.state.borrow_mut().finished = true;
    let got = rig.recv(id);
    assert!(rig.session.is_done());
    assert_eq!(got, [Got::Detached(DetachReason::HostClosing)]);
}

#[test]
fn a_signal_is_a_close_with_the_default_grace() {
    let mut rig = Rig::new();
    rig.spawned();
    rig.session.terminate(rig.now);
    assert_eq!(rig.state.borrow().closed, Some(Duration::from_secs(3)));
    rig.advance(Duration::from_secs(3 + 15));
    rig.session.pump(rig.now);
    assert!(rig.session.is_done(), "a hangup that never ends does not keep the host");
}

#[test]
fn an_exited_program_keeps_its_screen_until_a_week_without_clients() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.session.closed(id);
    let exit = wire::ExitInfo { code: 3, signal: 0, at_ms: 1 };
    rig.state.borrow_mut().exit = Some(exit);
    rig.session.pump(rig.now);
    let later = rig.connect();
    let got = rig.hello(later, Role::Controller, false);
    assert!(
        matches!(got.as_slice(), [Got::Welcome(w)] if w.state == HostState::Exited && w.exit == Some(exit))
    );
    let got = rig.attach(later, StreamMode::Sync, 0);
    let exited = got.iter().position(|g| matches!(g, Got::Event(_, e) if e.contains("Exited")));
    let repainted = got.iter().position(|g| matches!(g, Got::Repaint { last: true, .. }));
    assert!(exited > repainted && repainted.is_some(), "told after the repaint: {got:?}");
    rig.advance(Duration::from_secs(8 * 24 * 3600));
    rig.session.pump(rig.now);
    assert!(!rig.session.is_done(), "not while someone is connected");
    rig.session.closed(later);
    rig.session.pump(rig.now);
    assert!(rig.session.is_done());
}

#[test]
fn a_running_program_is_never_ended_by_a_timer() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.session.closed(id);
    rig.advance(Duration::from_secs(30 * 24 * 3600));
    let pump = rig.session.pump(rig.now);
    assert!(!rig.session.is_done());
    assert_eq!(pump.next, None, "detached and idle: no timer at all");
}

#[test]
fn an_overrun_cursor_is_cut_loose() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 0);
    // The client reads nothing: its frames stay queued, so the stream backs up in the ring.
    rig.session.pump(rig.now);
    let chunk = vec![b'y'; 1024 * 1024];
    for _ in 0..6 {
        rig.stream.append(&chunk);
        rig.session.pump(rig.now);
    }
    let got = rig.recv(id);
    assert!(got.contains(&Got::Detached(DetachReason::TooSlow)), "the ring gave up on it");
}

#[test]
fn detach_ends_the_connection_and_keeps_the_program() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.attach(id, StreamMode::Raw, 0);
    rig.send(id, &Message::Detach);
    rig.recv(id);
    assert!(rig.session.should_close(id));
    rig.session.closed(id);
    assert!(rig.state.borrow().closed.is_none());
    assert!(!rig.session.is_done());
}

#[test]
fn an_event_for_a_newer_host_is_answered_not_fatal() {
    let mut rig = Rig::new();
    let id = rig.spawned();
    rig.send(id, &Message::Event(wire::Event { offset: 0, kind: EventKind::Bell }));
    assert_eq!(rig.recv(id), [Got::Error(ty::EVENT, error_code::BAD_STATE)]);
    assert!(!rig.session.should_close(id));
}
