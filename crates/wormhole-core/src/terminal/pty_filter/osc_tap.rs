//! A second look at the PTY's bytes, for the OSC sequences alacritty ignores.
//!
//! alacritty_terminal parses the PTY on its own thread and dispatches only the OSCs it knows
//! (0/2/4/8/10/11/12/22/50/52/104/110/111/112); the rest it drops without a hook. So the bytes go
//! through a second `vte::Parser` here, on the way from the PTY to alacritty, with a `Perform`
//! that acts on `osc_dispatch` alone. It runs the state machine alacritty's parser runs, so both
//! see the same sequences with the same parameters and the same ends. What it finds goes to the
//! UI as events and shared state through `EventProxy`, never through the terminal's lock: the PTY
//! thread takes that lock a moment later to parse the same bytes.
//!
//! The bytes pass to alacritty unchanged, with one addition. The tap sees a prompt mark (OSC 133
//! `A`) before alacritty has drawn anything of the prompt, so it cannot know the row the prompt
//! lands on, and that row keeps moving: alacritty's lines shift as output scrolls and are dropped
//! once the history is full, which the grid doesn't count. So no positions are kept. Instead the
//! tap wraps each part of a command block in a hyperlink of its own (OSC 8, one id per prompt):
//! the prompt from its `A` to its `B` (URI `wormhole:prompt`), the command line from `B` to `C`
//! (`wormhole:command`) and the output from `C` to `D` (`wormhole:output`). alacritty stores
//! the link on the cells, where it scrolls, drops and clears with the text itself and goes when a
//! row is overwritten, and `marks` reads the blocks back from the grid when asked. A hyperlink
//! the shell or a program ends (`OSC 8 ;;`) re-arms the open mark, so a prompt or an output with
//! links of its own still carries it on the rest of its cells. Nothing draws hyperlinks today; a
//! renderer that does must skip these URIs (`marks::is_mark`).
//!
//! An injection sits right after the sequence that asked for it. After a BEL-terminated sequence
//! it ends in BEL; after one ended by ESC (`ESC \`, or whatever escape followed) it ends in ESC
//! too, so the original byte after it completes both the same way and nothing shows. A sequence
//! cut short by CAN or SUB gets none.
//!
//! Most output is text and CSI sequences (colors, cursor moves), which the tap has no use for, so
//! it doesn't run them through its parser while it knows the parser is in the ground state: there
//! only ESC starts anything, and a complete CSI sequence (`ESC [`, parameter and intermediate
//! bytes, a final byte) leaves it in the ground state again, whatever state it was in before. So
//! the tap skips text and complete CSI sequences while in the ground state, feeds everything else
//! to the parser, and knows it is back in the ground state at the end of the next complete CSI
//! sequence it fed. The parser ends up exactly where it would have been, and sees every OSC.
//!
//! vte with `std` grows its OSC buffer without bound, as alacritty's parser does. A string that
//! keeps this parser busy for more than [`QUIET_MAX`] bytes without anything coming out of it (an
//! OSC or APC payload) resets the parser, which drops that one sequence here and nothing else: an
//! ESC inside such a payload ends it for both parsers alike, so they stay in step.

use std::io;

use alacritty_terminal::vte::{Params, Parser, Perform};

use super::osc::{self, KittyChunks, Mark, Osc, PromptKind};
use crate::terminal::marks::{self, Part};

/// Bytes are fed in slices this long, so the quiet check happens at that pace.
const SLICE: usize = 4096;
/// A sequence still accumulating after this many bytes is dropped (see the module docs).
const QUIET_MAX: usize = 32 * 1024;

/// The tap: see the module docs.
pub(super) struct OscTap {
    parser: Parser,
    perform: Tap,
    /// Output with injections in it, not yet handed over: `out[taken..]`.
    out: Vec<u8>,
    taken: usize,
    /// Bytes fed since the parser last reported anything (a string being accumulated).
    quiet: usize,
    /// The parser is known to be in the ground state (see the module docs).
    ground: bool,
    /// Skip what the parser has no use for (off only to test that skipping changes nothing).
    skip: bool,
    /// Bytes handed to the parser, for the test that keeps most output away from it.
    #[cfg(test)]
    fed: usize,
}

/// The `Perform`: everything except `osc_dispatch` only notes that the parser said something.
#[derive(Default)]
struct Tap {
    kitty: KittyChunks,
    events: Vec<Osc>,
    /// A callback ran during the current slice.
    saw: bool,
    /// What to insert after the sequence just dispatched (stops `advance_until_terminated`).
    inject: Option<Inject>,
    /// The current prompt's number (0: none yet) and which of its parts is being marked.
    prompt_seq: u64,
    open: Option<Part>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Inject {
    /// Start marking this part of the prompt's block.
    Link(Part, u64),
    LinkEnd,
}

impl OscTap {
    pub(super) fn new() -> Self {
        Self {
            parser: Parser::new(),
            perform: Tap::default(),
            out: Vec::new(),
            taken: 0,
            quiet: 0,
            ground: true,
            skip: true,
            #[cfg(test)]
            fed: 0,
        }
    }

    /// Read through `inner`, parse what came, and hand it over with any injections in place.
    pub(super) fn read_from(
        &mut self,
        mut inner: impl FnMut(&mut [u8]) -> io::Result<usize>,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        if self.taken < self.out.len() {
            return Ok(self.hand_over(buf));
        }
        let n = inner(buf)?;
        if n == 0 {
            return Ok(0);
        }
        match self.filter(&buf[..n]) {
            // The usual case: nothing to add, the bytes stay where they are.
            None => Ok(n),
            Some(out) => {
                self.out = out;
                self.taken = 0;
                Ok(self.hand_over(buf))
            }
        }
    }

    /// What the parser found since the last call, in stream order.
    pub(super) fn drain_events(&mut self) -> std::vec::Drain<'_, Osc> {
        self.perform.events.drain(..)
    }

    fn hand_over(&mut self, buf: &mut [u8]) -> usize {
        let rest = &self.out[self.taken..];
        let n = rest.len().min(buf.len());
        buf[..n].copy_from_slice(&rest[..n]);
        self.taken += n;
        if self.taken == self.out.len() {
            self.out.clear();
            self.taken = 0;
        }
        n
    }

    /// Run `bytes` through the parser. `None` when they pass as they are; otherwise the bytes to
    /// hand over instead, the injections in place.
    fn filter(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        let mut out: Option<Vec<u8>> = None;
        // How much of `bytes` is in `out` already.
        let mut copied = 0;
        let mut at = 0;
        while at < bytes.len() {
            if self.skip && self.ground {
                at = skip_ground(bytes, at);
                self.quiet = 0;
                if at == bytes.len() {
                    break;
                }
                self.ground = false;
            }
            // Feed up to the end of the next complete CSI sequence, after which the parser is in the
            // ground state, or a slice when that is further.
            let (end, grounds) = match csi_after(bytes, at) {
                Some(end) if self.skip && end - at <= SLICE => (end, true),
                _ => ((at + SLICE).min(bytes.len()), false),
            };
            self.perform.saw = false;
            let used = self.parser.advance_until_terminated(&mut self.perform, &bytes[at..end]);
            #[cfg(test)]
            {
                self.fed += used;
            }
            self.quiet = if self.perform.saw { 0 } else { self.quiet + used };
            if self.quiet > QUIET_MAX {
                self.parser = Parser::new();
                self.quiet = 0;
                self.ground = true;
            }
            self.ground |= grounds && at + used == end;
            at += used;
            if let Some(inject) = self.perform.inject.take()
                && let Some(sequence) = injection(inject, bytes[at - 1])
            {
                let out = out.get_or_insert_with(|| Vec::with_capacity(bytes.len() + 2 * sequence.len()));
                out.extend_from_slice(&bytes[copied..at]);
                out.extend_from_slice(&sequence);
                copied = at;
            }
        }
        let mut out = out?;
        out.extend_from_slice(&bytes[copied..]);
        Some(out)
    }
}

/// Past the text and complete CSI sequences from `at` on, for a parser in the ground state: the
/// first ESC that starts anything else, or the end.
fn skip_ground(bytes: &[u8], mut at: usize) -> usize {
    loop {
        match memchr::memchr(0x1b, &bytes[at..]) {
            None => return bytes.len(),
            Some(k) => at += k,
        }
        match csi_end(bytes, at) {
            Some(end) => at = end,
            None => return at,
        }
    }
}

/// The end of the complete CSI sequence starting at `at`, if one does: `ESC [`, parameter and
/// intermediate bytes (0x20–0x3F), a final byte (0x40–0x7E). Anything else in it (a control, a
/// byte past ASCII) or the end of the bytes first, and it isn't one here.
fn csi_end(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at + 1) != Some(&b'[') {
        return None;
    }
    let mut end = at + 2;
    while end < bytes.len() && (0x20..=0x3f).contains(&bytes[end]) {
        end += 1;
    }
    (end < bytes.len() && (0x40..=0x7e).contains(&bytes[end])).then_some(end + 1)
}

/// The end of the first complete CSI sequence that starts at or after `at`.
fn csi_after(bytes: &[u8], mut at: usize) -> Option<usize> {
    loop {
        at += memchr::memchr(0x1b, &bytes[at..])?;
        if let Some(end) = csi_end(bytes, at) {
            return Some(end);
        }
        at += 1;
    }
}

/// The bytes to insert after a sequence that ended with `terminator`, or none after CAN/SUB.
fn injection(inject: Inject, terminator: u8) -> Option<Vec<u8>> {
    let end = match terminator {
        0x07 | 0x1b => terminator,
        _ => return None,
    };
    let mut sequence = match inject {
        Inject::Link(part, seq) => {
            format!("\x1b]8;id={};{}", marks::prompt_id(seq), marks::uri(part)).into_bytes()
        }
        Inject::LinkEnd => b"\x1b]8;;".to_vec(),
    };
    sequence.push(end);
    Some(sequence)
}

impl Tap {
    fn on_mark(&mut self, mark: Mark) {
        match mark {
            Mark::PromptStart(PromptKind::Initial) => {
                self.prompt_seq += 1;
                self.mark(Some(Part::Prompt));
            }
            // A secondary, right or continuation prompt belongs to the command of the last
            // initial one; before any, there is nothing to continue.
            Mark::PromptStart(_) if self.prompt_seq > 0 => self.mark(Some(Part::Prompt)),
            Mark::PromptStart(_) => {}
            // What the shell echoes of the typed command, until the command runs (`C`); a shell
            // that skips `C` leaves its output marked as the command.
            Mark::CommandStart if self.prompt_seq > 0 => self.mark(Some(Part::Command)),
            Mark::OutputStart if self.prompt_seq > 0 => self.mark(Some(Part::Output)),
            Mark::CommandStart | Mark::OutputStart | Mark::Finished(_) => self.mark(None),
        }
    }

    /// Mark `part` of the current prompt's block from here on (`None`: end the open mark).
    fn mark(&mut self, part: Option<Part>) {
        if part.is_none() && self.open.is_none() {
            return;
        }
        self.open = part;
        self.inject = Some(match part {
            Some(part) => Inject::Link(part, self.prompt_seq),
            None => Inject::LinkEnd,
        });
    }
}

impl Perform for Tap {
    fn print(&mut self, _: char) {
        self.saw = true;
    }

    fn execute(&mut self, _: u8) {
        self.saw = true;
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {
        self.saw = true;
    }

    fn put(&mut self, _: u8) {
        self.saw = true;
    }

    fn unhook(&mut self) {
        self.saw = true;
    }

    fn csi_dispatch(&mut self, _: &Params, _: &[u8], _: bool, _: char) {
        self.saw = true;
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        self.saw = true;
        // RIS, `ESC c`, resets everything, the program-reported status included. DECSTR (`CSI ! p`)
        // is a soft reset that keeps it, and arrives as a CSI sequence, not here.
        if byte == b'c' && intermediates.is_empty() && !ignore {
            self.events.push(Osc::FullReset);
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        self.saw = true;
        let Some(osc) = osc::parse(params, bell_terminated, &mut self.kitty) else { return };
        match osc {
            Osc::HyperlinkEnd => {
                if let Some(part) = self.open {
                    self.inject = Some(Inject::Link(part, self.prompt_seq));
                }
            }
            Osc::Mark(mark, _) => {
                self.on_mark(mark);
                self.events.push(Osc::Mark(mark, self.prompt_seq));
            }
            other => self.events.push(other),
        }
    }

    fn terminated(&self) -> bool {
        self.inject.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Read};
    use std::path::PathBuf;

    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

    use super::super::osc::{BODY_MAX_CHARS, Mark, Notification, Osc, PromptKind};
    use super::{OscTap, QUIET_MAX, SLICE};
    use crate::terminal::GridSize;
    use crate::terminal::marks::{Part, prompt_id, uri};
    use crate::terminal::program_status::{Report, State};

    /// Hands out fixed chunks, then reports `WouldBlock` like a drained PTY.
    struct Chunks(VecDeque<Vec<u8>>);

    impl Read for Chunks {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some(mut chunk) = self.0.pop_front() else {
                return Err(io::ErrorKind::WouldBlock.into());
            };
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            if n < chunk.len() {
                self.0.push_front(chunk.split_off(n));
            }
            Ok(n)
        }
    }

    /// Everything the tap hands over for these chunks, read with a buffer of `buf_len`, and
    /// what it reported.
    fn tapped(chunks: &[&[u8]], buf_len: usize) -> (Vec<u8>, Vec<Osc>) {
        tapped_by(OscTap::new(), chunks, buf_len)
    }

    fn tapped_by(mut tap: OscTap, chunks: &[&[u8]], buf_len: usize) -> (Vec<u8>, Vec<Osc>) {
        let mut inner = Chunks(chunks.iter().filter(|c| !c.is_empty()).map(|c| c.to_vec()).collect());
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut buf = vec![0u8; buf_len];
        loop {
            match tap.read_from(|b| inner.read(b), &mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => panic!("read failed: {err}"),
            }
            events.extend(tap.drain_events());
        }
        (out, events)
    }

    fn term(bytes: &[u8]) -> Term<VoidListener> {
        let size = GridSize { cols: 40, rows: 5, cell_width: 8.0, cell_height: 16.0 };
        let mut term = Term::new(Config { scrolling_history: 100, ..Config::default() }, &size, VoidListener);
        Processor::<StdSyncHandler>::new().advance(&mut term, bytes);
        term
    }

    fn screen_text(term: &Term<VoidListener>) -> String {
        let grid = term.grid();
        (0..grid.screen_lines())
            .map(|line| {
                (0..grid.columns()).map(|col| grid[Line(line as i32)][Column(col)].c).collect::<String>()
            })
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    /// The block mark on each cell of `line` that has one, as (column, prompt number, part).
    fn marks_on(term: &Term<VoidListener>, line: i32) -> Vec<(usize, u64, Part)> {
        let row = &term.grid()[Line(line)];
        (0..row.len())
            .filter_map(|col| {
                let link = row[Column(col)].hyperlink()?;
                let part = [Part::Prompt, Part::Command, Part::Output]
                    .into_iter()
                    .find(|p| uri(*p) == link.uri())?;
                let seq = link.id().strip_prefix("wormhole-prompt-")?.parse().ok()?;
                Some((col, seq, part))
            })
            .collect()
    }

    /// `marks_on` as runs: (first column, count, prompt number, part).
    fn runs_on(term: &Term<VoidListener>, line: i32) -> Vec<(usize, usize, u64, Part)> {
        let mut runs: Vec<(usize, usize, u64, Part)> = Vec::new();
        for (col, seq, part) in marks_on(term, line) {
            match runs.last_mut() {
                Some(run) if run.0 + run.1 == col && run.2 == seq && run.3 == part => run.1 += 1,
                _ => runs.push((col, 1, seq, part)),
            }
        }
        runs
    }

    const PROMPT: &[u8] = b"\x1b]133;A\x07$ \x1b]133;B\x07";

    #[test]
    fn bytes_without_marks_pass_unchanged_and_are_reported() {
        let input = b"\x1b[32mok\x1b[0m\r\n\x1b]7;file://host/tmp\x07\x1b]9;Built\x1b\\plain\x1b]2;title\x07";
        for buf_len in [7, 64, 4096] {
            let (out, events) = tapped(&[input], buf_len);
            assert_eq!(out, input, "buffer {buf_len}");
            assert_eq!(
                events,
                [
                    Osc::Cwd { host: "host".into(), path: PathBuf::from("/tmp") },
                    Osc::Notify(Notification { title: String::new(), body: "Built".into() }),
                ],
                "buffer {buf_len}"
            );
        }
    }

    #[test]
    fn program_status_sequences_are_reported_in_order_whatever_ends_them() {
        let input = b"\x1b]7501;?\x1b\\\x1b[c\x1b]7501;state=working:app=x\x07\x1b]7501;state=clear\x1b\\";
        for buf_len in [3, 64, 4096] {
            let (out, events) = tapped(&[input], buf_len);
            assert_eq!(out, input, "buffer {buf_len}: the bytes pass unchanged");
            assert!(
                matches!(
                    &events[..],
                    [
                        Osc::ProgramStatusQuery,
                        Osc::ProgramStatus(Report::Set { id: None, record }),
                        Osc::ProgramStatus(Report::Clear { id: None }),
                    ] if record.state == State::Working && record.app.as_deref() == Some("x")
                ),
                "buffer {buf_len}: {events:?}"
            );
        }
        // Split across reads anywhere, it is still read once.
        let chunks: Vec<&[u8]> = input.chunks(5).collect();
        assert_eq!(tapped(&chunks, 64).1.len(), 3);
    }

    #[test]
    fn a_report_over_the_sequence_limit_is_ignored_by_its_terminator() {
        // 4096 bytes in all with ESC \ is the longest legal sequence; BEL saves a byte.
        let pad = |body_len: usize| format!("state=idle:x={}", "y".repeat(body_len - "state=idle:x=".len()));
        let sequence = |body: &str, end: &[u8]| [b"\x1b]7501;", body.as_bytes(), end].concat();
        let count = |bytes: Vec<u8>| tapped(&[&bytes], 64).1.len();
        assert_eq!(count(sequence(&pad(4096 - 7 - 2), b"\x1b\\")), 1);
        assert_eq!(count(sequence(&pad(4096 - 7 - 2 + 1), b"\x1b\\")), 0);
        assert_eq!(count(sequence(&pad(4096 - 7 - 1), b"\x07")), 1);
        assert_eq!(count(sequence(&pad(4096 - 7 - 1 + 1), b"\x07")), 0);
    }

    #[test]
    fn a_full_reset_is_reported_and_a_soft_reset_is_not() {
        // RIS is `ESC c`. DECSTR (`CSI ! p`), a charset designation that ends in `c` (`ESC ( c`)
        // and a `c` inside a CSI sequence or text are not.
        let (out, events) = tapped(&[b"a\x1bcb"], 64);
        assert_eq!((out.as_slice(), &events[..]), (&b"a\x1bcb"[..], &[Osc::FullReset][..]));
        let (_, events) = tapped(&[b"\x1b[!p\x1b(c\x1b)c\x1b[c\x1b[0cc\x1b]0;\x07c"], 64);
        assert_eq!(events, []);
        // Cut across reads, still once.
        assert_eq!(tapped(&[b"\x1b", b"c"], 64).1, [Osc::FullReset]);
    }

    /// A small seeded generator (xorshift), so a failure can be replayed.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }

        fn pick<'a>(&mut self, items: &[&'a [u8]]) -> &'a [u8] {
            items[self.below(items.len())]
        }
    }

    /// Output a program might send, and some it shouldn't: text, colors, cursor moves, marks,
    /// links, other strings, broken sequences and bytes.
    fn random_output(rng: &mut Rng, pieces: usize) -> Vec<u8> {
        const TEXT: &[&[u8]] = &[
            b"hello ",
            b"cargo build",
            b"\r\n",
            b"\t",
            "日本語 ".as_bytes(),
            "é".as_bytes(),
            "👍🏽".as_bytes(),
        ];
        const CSI: &[&[u8]] = &[
            b"\x1b[0m",
            b"\x1b[38;2;10;20;30m",
            b"\x1b[1;31m",
            b"\x1b[2J",
            b"\x1b[?2026h",
            b"\x1b[?2026l",
            b"\x1b[12;40H",
            b"\x1b[ q",
            b"\x1b[>4;1m",
            b"\x1b[38:2::1:2:3m",
            b"\x1b[!p",
        ];
        const MARKS: &[&[u8]] = &[
            b"\x1b]133;A\x07",
            b"\x1b]133;B\x1b\\",
            b"\x1b]133;C\x07",
            b"\x1b]133;D;0\x1b\\",
            b"\x1b]133;A;k=s\x07",
            b"\x1b]133;P;k=i\x07",
        ];
        const OTHER: &[&[u8]] = &[
            b"\x1b]8;;https://example.com\x1b\\",
            b"\x1b]8;;\x1b\\",
            b"\x1b]0;title\x07",
            b"\x1b]9;note\x07",
            b"\x1b]777;notify;a;b\x07",
            b"\x1b]7501;state=working:app=x:msg=aGk=\x07",
            b"\x1b]7501;?\x1b\\",
            b"\x1bc",
            b"\x1bP+q544e\x1b\\",
            b"\x1b_Gf=24,s=1,v=1;AAAA\x1b\\",
            b"\x1b7",
            b"\x1b8",
            b"\x1b(B",
            b"\x1b",
            b"\x1b[",
            b"\x1b[12",
            b"\x1b]133;",
            b"\x18",
            b"\x1a",
            b"\x07",
            b"\xff",
            b"\xe2\x82",
            b"\x9b",
            b"\x1b[1\x0a2m",
            b"\x1b[3\x1b[4m",
            b"\x1b]8;;x\x1b[31m",
            b"\x1b[\xc3\xa9m",
        ];
        let mut out = Vec::new();
        for _ in 0..pieces {
            let piece = match rng.below(10) {
                0..=3 => rng.pick(TEXT),
                4..=6 => rng.pick(CSI),
                7 => rng.pick(MARKS),
                _ => rng.pick(OTHER),
            };
            out.extend_from_slice(piece);
        }
        out
    }

    #[test]
    fn skipping_text_and_csi_changes_nothing_the_tap_reports_or_hands_over() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for case in 0..3000 {
            let pieces = 1 + rng.below(80);
            let bytes = random_output(&mut rng, pieces);
            // Cut into reads anywhere, sequences and characters included.
            let mut chunks = Vec::new();
            let mut at = 0;
            while at < bytes.len() {
                let n = 1 + rng.below(40);
                chunks.push(&bytes[at..(at + n).min(bytes.len())]);
                at += n;
            }
            let buf_len = [7, 64, 4096][rng.below(3)];
            let mut plain = OscTap::new();
            plain.skip = false;
            let expected = tapped_by(plain, &chunks, buf_len);
            let got = tapped_by(OscTap::new(), &chunks, buf_len);
            assert_eq!(got, expected, "case {case}: {:?}", String::from_utf8_lossy(&bytes));
        }
    }

    /// How much of `bytes`, read in 64 KiB chunks, reaches the tap's parser.
    fn share_fed(bytes: &[u8]) -> f64 {
        let mut tap = OscTap::new();
        for chunk in bytes.chunks(64 * 1024) {
            let _ = tap.filter(chunk);
        }
        tap.fed as f64 / bytes.len() as f64
    }

    #[test]
    fn colored_output_and_plain_text_mostly_bypass_the_parser() {
        let mut colored = Vec::new();
        let mut plain = Vec::new();
        for i in 0..4000 {
            colored.extend_from_slice(
                format!(
                    "\x1b[38;2;{};80;200;48;2;10;{};30m▀\x1b[1;32m+ added\x1b[0m line {i}\r\n",
                    i % 256,
                    i % 199
                )
                .as_bytes(),
            );
            plain.extend_from_slice(
                format!("2026-10-10T12:00:{:02}Z INFO request id={i:06x} done\r\n", i % 60).as_bytes(),
            );
        }
        // A prompt mark now and then still goes through the parser, and is seen.
        colored.extend_from_slice(b"\x1b]133;A\x07$ ");
        assert!(share_fed(&colored) < 0.02, "colored output fed: {:.3}", share_fed(&colored));
        assert!(share_fed(&plain) < 0.001, "plain text fed: {:.3}", share_fed(&plain));
        let (_, events) = tapped(&[&colored], 4096);
        assert_eq!(events.len(), 1, "the mark at the end is still reported: {events:?}");
    }

    #[test]
    fn a_sequence_split_across_reads_is_read_once() {
        let bytes = b"ab\x1b]133;D;3\x07cd\x1b]133;A\x1b\\$ ";
        for split in 0..=bytes.len() {
            let (a, b) = bytes.split_at(split);
            let (out, events) = tapped(&[a, b], 64);
            assert_eq!(
                events,
                [Osc::Mark(Mark::Finished(Some(3)), 0), Osc::Mark(Mark::PromptStart(PromptKind::Initial), 1)],
                "split at {split}"
            );
            assert_eq!(screen_text(&term(&out)), "abcd$", "split at {split}");
        }
        // Byte by byte, with a byte-sized buffer.
        let chunks: Vec<&[u8]> = bytes.chunks(1).collect();
        let (out, events) = tapped(&chunks, 1);
        assert_eq!(events.len(), 2);
        assert_eq!(screen_text(&term(&out)), "abcd$");
    }

    #[test]
    fn a_prompt_is_marked_on_its_cells_whatever_ends_the_sequences() {
        let bel = PROMPT;
        let st = b"\x1b]133;A\x1b\\$ \x1b]133;B\x1b\\";
        let mixed = b"\x1b]133;A\x1b\\$ \x1b]133;B\x07";
        for (name, input) in [("BEL", bel), ("ST", &st[..]), ("mixed", &mixed[..])] {
            let (out, _) = tapped(&[input, b"ls\r\nout\r\n"], 64);
            let screen = term(&out);
            assert_eq!(screen_text(&screen), "$ ls\nout", "{name}: the screen shows what the program wrote");
            assert_eq!(runs_on(&screen, 0), [(0, 2, 1, Part::Prompt), (2, 2, 1, Part::Command)], "{name}");
            // Without a `C`, what follows the command line still reads as the command.
            assert_eq!(runs_on(&screen, 1), [(0, 3, 1, Part::Command)], "{name}");
            // The parser is where alacritty's is: a following escape still works.
            let (out, _) = tapped(&[input, b"\x1b[2Cx"], 64);
            assert_eq!(screen_text(&term(&out)), "$   x", "{name}");
        }
        // An ESC that starts another sequence ends the mark as ST would.
        let (out, _) = tapped(&[b"\x1b]133;A\x1b[1m$\x1b]133;B\x1b[0m "], 64);
        let screen = term(&out);
        assert_eq!(screen_text(&screen), "$");
        assert_eq!(marks_on(&screen, 0), [(0, 1, Part::Prompt), (1, 1, Part::Command)]);
        assert!(
            screen.grid()[Line(0)][Column(0)].flags.contains(alacritty_terminal::term::cell::Flags::BOLD)
        );
    }

    #[test]
    fn a_block_is_marked_prompt_command_and_output_and_ends_at_d() {
        let input = b"\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07out\r\nmore\r\n\x1b]133;D;0\x07%\r\n\x1b]133;A\x07$ \x1b]133;B\x07";
        let (out, events) = tapped(&[input], 64);
        let term = term(&out);
        assert_eq!(screen_text(&term), "$ ls\nout\nmore\n%\n$");
        assert_eq!(runs_on(&term, 0), [(0, 2, 1, Part::Prompt), (2, 2, 1, Part::Command)]);
        assert_eq!(runs_on(&term, 1), [(0, 3, 1, Part::Output)]);
        assert_eq!(runs_on(&term, 2), [(0, 4, 1, Part::Output)]);
        assert!(runs_on(&term, 3).is_empty(), "after `D` nothing is marked until the next prompt");
        assert_eq!(runs_on(&term, 4), [(0, 2, 2, Part::Prompt)]);
        // Each mark is reported with its prompt's number; the finish belongs to the command's.
        let marks: Vec<(Mark, u64)> = events
            .into_iter()
            .filter_map(|e| if let Osc::Mark(m, seq) = e { Some((m, seq)) } else { None })
            .collect();
        assert_eq!(
            marks,
            [
                (Mark::PromptStart(PromptKind::Initial), 1),
                (Mark::CommandStart, 1),
                (Mark::OutputStart, 1),
                (Mark::Finished(Some(0)), 1),
                (Mark::PromptStart(PromptKind::Initial), 2),
                (Mark::CommandStart, 2),
            ]
        );
    }

    #[test]
    fn each_prompt_gets_its_own_id_and_extra_prompt_lines_share_it() {
        let input = b"\x1b]133;A\x07$ \x1b]133;B\x07for\r\n\x1b]133;A;k=s\x07> \x1b]133;B\x07done\r\n\x1b]133;C\x07x\r\n\x1b]133;D;0\x07\x1b]133;A;k=i\x07$ \x1b]133;B\x07";
        let term = term(&tapped(&[input], 64).0);
        assert_eq!(screen_text(&term), "$ for\n> done\nx\n$");
        assert_eq!(runs_on(&term, 0), [(0, 2, 1, Part::Prompt), (2, 3, 1, Part::Command)]);
        assert_eq!(
            runs_on(&term, 1),
            [(0, 2, 1, Part::Prompt), (2, 4, 1, Part::Command)],
            "the secondary prompt and the command's second line"
        );
        assert_eq!(runs_on(&term, 2), [(0, 1, 1, Part::Output)]);
        assert_eq!(runs_on(&term, 3), [(0, 2, 2, Part::Prompt)]);
    }

    #[test]
    fn a_shell_that_skips_b_has_its_prompt_mark_closed_at_c() {
        let (out, _) = tapped(&[b"\x1b]133;A\x07$ cmd\r\n\x1b]133;C\x07output\r\n"], 64);
        let term = term(&out);
        assert_eq!(runs_on(&term, 0), [(0, 5, 1, Part::Prompt)]);
        assert_eq!(runs_on(&term, 1), [(0, 6, 1, Part::Output)]);
    }

    #[test]
    fn the_shells_own_links_in_the_prompt_leave_the_mark_on_the_rest() {
        let input = b"\x1b]133;A\x07\x1b]8;;file:///Users/me\x07~me\x1b]8;;\x07 $ \x1b]133;B\x07ls";
        let term = term(&tapped(&[input], 64).0);
        assert_eq!(screen_text(&term), "~me $ ls");
        let row = &term.grid()[Line(0)];
        assert_eq!(row[Column(0)].hyperlink().map(|l| l.uri().to_string()), Some("file:///Users/me".into()));
        assert_eq!(runs_on(&term, 0), [(3, 3, 1, Part::Prompt), (6, 2, 1, Part::Command)]);
    }

    #[test]
    fn a_programs_links_in_the_output_leave_the_mark_on_the_rest() {
        let input = b"\x1b]133;A\x07$ \x1b]133;B\x07x\r\n\x1b]133;C\x07see \x1b]8;;https://e.com\x07here\x1b]8;;\x07 now\r\n";
        let term = term(&tapped(&[input], 64).0);
        assert_eq!(screen_text(&term), "$ x\nsee here now");
        let row = &term.grid()[Line(1)];
        assert_eq!(row[Column(4)].hyperlink().map(|l| l.uri().to_string()), Some("https://e.com".into()));
        assert_eq!(runs_on(&term, 1), [(0, 4, 1, Part::Output), (8, 4, 1, Part::Output)]);
        assert_eq!(prompt_id(1), "wormhole-prompt-1");
    }

    #[test]
    fn a_sequence_cut_short_gets_no_injection() {
        let (out, events) = tapped(&[b"\x1b]133;A\x18$ \x1b]133;B\x1a"], 64);
        assert_eq!(out, b"\x1b]133;A\x18$ \x1b]133;B\x1a");
        assert_eq!(events.len(), 2, "vte still reports them, as alacritty's parser sees them");
    }

    #[test]
    fn an_oversized_sequence_is_dropped_and_the_parser_recovers() {
        // The check runs per slice, so a payload a slice longer than the cap is certainly over it.
        let mut big = b"\x1b]9;".to_vec();
        big.extend(std::iter::repeat_n(b'a', QUIET_MAX + 2 * SLICE));
        big.extend_from_slice(b"\x07\x1b]133;D;1\x07");
        let (out, events) = tapped(&[&big], 4096);
        assert_eq!(out, big, "the bytes still reach alacritty");
        assert_eq!(events, [Osc::Mark(Mark::Finished(Some(1)), 0)]);
        // A long APC string (kitty's graphics) is ignored by both parsers; an OSC right after it
        // is still seen.
        let mut apc = b"\x1b_G".to_vec();
        apc.extend(std::iter::repeat_n(b'A', QUIET_MAX + 2 * SLICE));
        apc.extend_from_slice(b"\x1b\\\x1b]133;D;2\x07");
        assert_eq!(tapped(&[&apc], 4096).1, [Osc::Mark(Mark::Finished(Some(2)), 0)]);
        // Just under the cap, a notification still comes through (its text cut to size).
        let mut fits = b"\x1b]9;".to_vec();
        fits.extend(std::iter::repeat_n(b'b', QUIET_MAX - SLICE));
        fits.extend_from_slice(b"\x07");
        let events = tapped(&[&fits], 4096).1;
        assert!(matches!(&events[..], [Osc::Notify(n)] if n.body.chars().count() == BODY_MAX_CHARS + 1));
    }

    #[test]
    fn an_escape_inside_a_notification_ends_it_as_alacritty_would() {
        let (_, events) = tapped(&[b"\x1b]777;notify;Title\x1b[31m;body\x07"], 64);
        assert_eq!(events, [Osc::Notify(Notification { title: "Title".into(), body: String::new() })]);
    }

    #[test]
    fn injections_longer_than_the_buffer_come_out_over_several_reads() {
        for buf_len in 1..8 {
            let (out, _) = tapped(&[PROMPT, b"ls"], buf_len);
            let term = term(&out);
            assert_eq!(screen_text(&term), "$ ls", "buffer {buf_len}");
            assert_eq!(
                runs_on(&term, 0),
                [(0, 2, 1, Part::Prompt), (2, 2, 1, Part::Command)],
                "buffer {buf_len}"
            );
        }
    }

    /// Throughput of the tap over 64 KiB chunks of colored output (it runs on every PTY read).
    /// Run with `--ignored --nocapture`.
    #[test]
    #[ignore = "measurement; run with --ignored --nocapture"]
    fn tap_throughput() {
        let mut chunk = Vec::new();
        while chunk.len() < 64 * 1024 {
            chunk.extend_from_slice(b"\x1b[32m+ added\x1b[0m line of \x1b[1mdiff\x1b[0m output\r\n");
        }
        chunk.truncate(64 * 1024);
        let mut tap = OscTap::new();
        let rounds = 2000;
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            assert!(tap.filter(std::hint::black_box(&chunk)).is_none());
        }
        let per_chunk = start.elapsed() / rounds;
        let gbps = (64.0 * 1024.0) / per_chunk.as_secs_f64() / 1e9;
        eprintln!("tap: {per_chunk:?} per 64 KiB chunk, {gbps:.2} GB/s");
    }
}
