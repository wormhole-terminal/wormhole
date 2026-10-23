//! Shared by the test files: a deterministic generator of every message, a reader that hands out
//! bytes in chosen pieces, and small helpers. No dependency: the generator is a xorshift and
//! every test prints its seed, so a failure is reproducible.
#![allow(dead_code)] // each test binary uses a part

use std::io::{self, Read};

use wormhole_wire::*;

pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub(crate) fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    pub(crate) fn usize(&mut self, n: usize) -> usize {
        self.below(n as u64) as usize
    }
    pub(crate) fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
    pub(crate) fn u8(&mut self) -> u8 {
        self.next() as u8
    }
    pub(crate) fn u16(&mut self) -> u16 {
        self.next() as u16
    }
    pub(crate) fn u32(&mut self) -> u32 {
        self.next() as u32
    }
    pub(crate) fn fill(&mut self, out: &mut [u8]) {
        for b in out {
            *b = self.u8();
        }
    }
    /// Random bytes, a random number of them below `max`.
    pub(crate) fn noise(&mut self, max: usize) -> Vec<u8> {
        let n = self.usize(max);
        self.bytes(n)
    }
    pub(crate) fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0; n];
        self.fill(&mut v);
        v
    }
}

/// Strings and bytes the generated messages borrow from.
pub(crate) struct Pool {
    pub strings: Vec<String>,
    pub data: Vec<u8>,
}

impl Pool {
    pub(crate) fn new(rng: &mut Rng) -> Self {
        let alphabet: Vec<char> = "abcXYZ019 _-/.:~é€日本🙂\u{0}\n".chars().collect();
        let mut strings = Vec::new();
        for i in 0..24 {
            let len = match i % 6 {
                0 => 0,
                1 => 1,
                5 => 200 + rng.usize(300),
                _ => rng.usize(40),
            };
            strings.push((0..len).map(|_| alphabet[rng.usize(alphabet.len())]).collect());
        }
        Self { strings, data: rng.bytes(MAX_DATA + 1) }
    }

    fn string<'a>(&'a self, rng: &mut Rng) -> &'a str {
        &self.strings[rng.usize(self.strings.len())]
    }

    /// A slice of data, mostly small, sometimes empty or a full chunk.
    fn bytes<'a>(&'a self, rng: &mut Rng) -> &'a [u8] {
        let len = match rng.below(10) {
            0 => 0,
            1 => MAX_DATA,
            2 => MAX_DATA - 1,
            3..=6 => rng.usize(100),
            _ => rng.usize(5000),
        };
        let from = rng.usize(self.data.len() - len);
        &self.data[from..from + len]
    }
}

fn size(rng: &mut Rng) -> Size {
    Size { cols: rng.u16(), rows: rng.u16(), cell_w: rng.u16(), cell_h: rng.u16() }
}

fn version(rng: &mut Rng) -> AppVersion {
    AppVersion { major: rng.u16(), minor: rng.u16(), patch: rng.u16() }
}

fn counters(rng: &mut Rng) -> Counters {
    Counters {
        epoch: rng.u32(),
        history_lines: rng.u32(),
        trimmed_total: rng.next(),
        screen_rows: rng.u16(),
        alt_screen: rng.bool(),
        revision: rng.next(),
    }
}

fn colors<'a>(rng: &mut Rng) -> Colors<'a> {
    let rgb = |rng: &mut Rng| [rng.u8(), rng.u8(), rng.u8()];
    let max = if rng.bool() { 17 } else { MAX_PALETTE + 1 };
    let n = rng.usize(max);
    Colors {
        fg: rgb(rng),
        bg: rgb(rng),
        cursor: rng.bool().then(|| rgb(rng)),
        palette: List::Owned((0..n).map(|_| rgb(rng)).collect()),
    }
}

fn exit(rng: &mut Rng) -> ExitInfo {
    ExitInfo { code: rng.u32() as i32, signal: rng.u8(), at_ms: rng.next() }
}

fn event<'a>(rng: &mut Rng, p: &'a Pool) -> Event<'a> {
    let kind = match rng.below(14) {
        0 => EventKind::Title(p.string(rng)),
        1 => EventKind::ResetTitle,
        2 => EventKind::Bell,
        3 => EventKind::Cwd(p.string(rng)),
        4 => EventKind::Mark(Mark {
            kind: MarkKind::from_wire(rng.u8() % 6),
            block: rng.next(),
            exit_code: rng.bool().then(|| rng.u32() as i32),
            at_ms: rng.next(),
        }),
        5 => EventKind::Notify { source: rng.u8(), title: p.string(rng), body: p.string(rng) },
        6 => EventKind::Progress { state: ProgressState::from_wire(rng.u8() % 7), percent: rng.u8() },
        7 => EventKind::ProgramStatus(p.string(rng)),
        8 => EventKind::ClipboardStore { selection: rng.u8(), data: p.bytes(rng) },
        9 => EventKind::ClipboardLoad { ticket: rng.u32(), selection: rng.u8() },
        10 => EventKind::WindowOp { op: rng.u8(), a: rng.u32(), b: rng.u32() },
        11 => EventKind::FullReset,
        12 => EventKind::RepaintSeeds { block: rng.u32(), link_seq: rng.next(), image_handle: rng.next() },
        _ => EventKind::Exited(exit(rng)),
    };
    Event { offset: rng.next(), kind }
}

fn welcome(rng: &mut Rng) -> Welcome {
    let mut id = [0; 12];
    rng.fill(&mut id);
    let mut token = [0; 32];
    rng.fill(&mut token);
    Welcome {
        proto: rng.u16(),
        proto_minor: rng.u16(),
        state: HostState::from_wire(rng.u8() % 4),
        host_version: version(rng),
        pid: rng.u32(),
        session_id: id,
        features: Features(rng.next()),
        started_at_ms: rng.next(),
        first_block_id: rng.next(),
        size: size(rng),
        counters: counters(rng),
        exit: rng.bool().then(|| exit(rng)),
        token: rng.bool().then_some(token),
    }
}

/// A random message of any kind, with every field random, borrowing from `p`.
pub(crate) fn message<'a>(rng: &mut Rng, p: &'a Pool) -> Message<'a> {
    match rng.below(35) {
        0 => {
            let min = 1 + rng.u16() % 100;
            Message::Hello(Hello {
                proto_min: min,
                proto_max: min + rng.u16() % 100,
                proto_minor: rng.u16(),
                role: Role::from_wire(rng.u8() % 5),
                force: rng.bool(),
                features: Features(rng.next()),
                client_version: version(rng),
                client_name: p.string(rng),
                ticket: rng.bool().then(|| {
                    let b = p.bytes(rng);
                    &b[..b.len().min(MAX_TICKET)]
                }),
            })
        }
        1 => Message::Welcome(welcome(rng)),
        2 => Message::Incompatible(Incompatible {
            host_min: rng.u16(),
            host_max: rng.u16(),
            host_version: version(rng),
        }),
        3 => Message::GetInfo,
        4 => Message::Info(welcome(rng)),
        5 => Message::Kill,
        6 => Message::Detach,
        7 => Message::Detached(DetachReason::from_wire(rng.u8() % 7)),
        8 => Message::Ping(rng.next()),
        9 => Message::Pong { n: rng.next(), pty_progress: rng.next() },
        10 => Message::Error(ErrorMsg { ref_type: rng.u8(), code: rng.u16(), text: p.string(rng) }),
        11 => {
            let args = (0..rng.usize(6)).map(|_| p.string(rng)).collect();
            let env = (0..rng.usize(6)).map(|_| (p.string(rng), p.string(rng))).collect();
            Message::Spawn(Spawn {
                size: size(rng),
                options: Options { scrollback_lines: rng.u32(), flags: rng.u32() },
                program: p.string(rng),
                cwd: p.string(rng),
                args: List::Owned(args),
                env: List::Owned(env),
                colors: rng.bool().then(|| colors(rng)),
            })
        }
        12 => Message::Attach(Attach {
            stream: StreamMode::from_wire(rng.u8() % 5),
            history_lines: rng.u32(),
            window: rng.u32(),
            resume_from: rng.bool().then(|| rng.next()),
        }),
        13 => Message::Resize(Resize { req: rng.u32(), size: size(rng) }),
        14 => Message::SetColors(colors(rng)),
        15 => Message::SetLimits(Limits { history_bytes: rng.next(), ring_bytes: rng.next() }),
        16 => Message::SetOptions(Options { scrollback_lines: rng.u32(), flags: rng.u32() }),
        17 => Message::Close { grace_ms: rng.u32() },
        18 => Message::ClipboardReply(ClipboardReply { ticket: rng.u32(), data: p.bytes(rng) }),
        19 => Message::Hook(Hook { name: p.string(rng), payload: p.bytes(rng) }),
        20 => Message::Input(p.bytes(rng)),
        21 => Message::InputAck(rng.next()),
        22 | 23 => Message::Out(Out { offset: rng.next(), bytes: p.bytes(rng) }),
        24 => {
            let bytes = p.bytes(rng);
            let (mut first, mut last) = (rng.bool(), rng.bool());
            if !first && !last && bytes.is_empty() {
                (first, last) = (true, true);
            }
            Message::Repaint(Repaint {
                epoch: rng.u32(),
                offset: rng.next(),
                kind: RepaintKind::from_wire(rng.u8() % 4),
                history_gap: rng.bool(),
                first,
                last,
                bytes,
            })
        }
        25 => Message::ResizeApplied(ResizeApplied {
            offset: rng.next(),
            epoch: rng.u32(),
            req: rng.u32(),
            size: size(rng),
        }),
        26 => Message::Ack(rng.next()),
        27 => Message::Counters(counters(rng)),
        28 => Message::Event(event(rng, p)),
        29 => Message::Backlog(List::Owned((0..rng.usize(20)).map(|_| event(rng, p)).collect())),
        30 => Message::PageGet(PageGet { epoch: rng.u32(), from: rng.next(), count: rng.u32() }),
        31 => {
            Message::Page(Page { epoch: rng.u32(), from: rng.next(), count: rng.u32(), bytes: p.bytes(rng) })
        }
        32 => {
            if rng.bool() {
                Message::FindStart(FindStart {
                    id: rng.u32(),
                    flags: rng.u8(),
                    direction: FindDirection::from_wire(rng.u8() % 3),
                    epoch: rng.u32(),
                    anchor_line: rng.next(),
                    anchor_col: rng.u32(),
                    pattern: p.string(rng),
                })
            } else {
                let matches = (0..rng.usize(30))
                    .map(|_| FindMatch {
                        line: rng.next(),
                        col: rng.u32(),
                        cells: rng.u32(),
                        text: p.string(rng),
                    })
                    .collect();
                Message::FindResults(FindResults {
                    id: rng.u32(),
                    epoch: rng.u32(),
                    more: rng.bool(),
                    matches: List::Owned(matches),
                })
            }
        }
        33 => Message::SetColorScheme(ColorScheme::from_wire(rng.u8() % 4)),
        _ => Message::FindCancel { id: rng.u32() },
    }
}

/// A reader that gives out at most `step()` bytes per call, to cut a stream anywhere.
pub(crate) struct Chunked<'a, F: FnMut() -> usize> {
    pub data: &'a [u8],
    pub step: F,
}

impl<F: FnMut() -> usize> Read for Chunked<'_, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = (self.step)().max(1).min(buf.len()).min(self.data.len());
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

/// Every frame in `stream`, decoded with reads of the sizes `step` gives, rendered with `Debug` so
/// the result can be compared across ways of cutting the same bytes. Stops at the first error and
/// includes it.
pub(crate) fn decode_all(stream: &[u8], step: impl FnMut() -> usize) -> Vec<String> {
    let mut src = Chunked { data: stream, step };
    let mut dec = Decoder::new();
    let mut out = Vec::new();
    loop {
        loop {
            match dec.next_frame() {
                Ok(Some(f)) => out.push(format!("{f:?}")),
                Ok(None) => break,
                Err(e) => {
                    out.push(format!("error {e:?}"));
                    if e.is_fatal() {
                        return out;
                    }
                }
            }
        }
        match dec.fill(&mut src) {
            Ok(0) | Err(_) => return out,
            Ok(_) => {}
        }
    }
}

pub(crate) fn encode_one(channel: u16, msg: &Message<'_>) -> Vec<u8> {
    let mut buf = Vec::new();
    encode(&mut buf, channel, 0, msg).expect("encodes");
    buf
}

/// A frame built by hand, for bytes the encoder would never write.
pub(crate) fn raw_frame(ty: u8, flags: u8, channel: u16, payload: &[u8]) -> Vec<u8> {
    let mut v = Header { len: payload.len() as u32, ty, flags, channel }.to_bytes().to_vec();
    v.extend_from_slice(payload);
    v
}

/// The payload of an encoded frame.
pub(crate) fn payload_of(frame: &[u8]) -> &[u8] {
    &frame[HEADER_LEN..]
}

/// The first result of decoding `bytes` pushed whole.
pub(crate) fn first(bytes: &[u8]) -> Result<Option<String>, WireError> {
    let mut dec = Decoder::new();
    dec.push(bytes);
    dec.next_frame().map(|f| f.map(|f| format!("{f:?}")))
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex")).collect()
}
