//! Shared by the fuzz targets: a byte string read as a script for a terminal.
//!
//! The first bytes choose the terminal (size, history, whether a screen checksum is answered);
//! the rest is the stream the program sends, with the host's calls mixed in: `0xFF` and an opcode
//! (then its arguments) is a call, `0xFF 0xFF` is a literal `0xFF`. A byte string made of plain
//! escape sequences is a valid script (no `0xFF` in it), so the `.vtt` cases and the recorded
//! sessions seed the corpus as they are, and the fuzzer's splices of them stay meaningful.

use wormhole_vt::image::ImageHandle;
use wormhole_vt::passthrough::Passthrough;
use wormhole_vt::{
    BlockPart, Cell, Column, Damage, Event, Limits, Line, Point, Policy, RowExtras, Scroll, SelectionType,
    Side, Terminal, TextCopy, TextShape, ViewRow,
};

/// Reads the script's bytes, 0 once they run out.
pub struct Bytes<'a>(pub &'a [u8]);

impl Bytes<'_> {
    pub fn byte(&mut self) -> u8 {
        let (&byte, rest) = self.0.split_first().unwrap_or((&0, &[]));
        self.0 = rest;
        byte
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The rest, cut into frames at every single `0xFF`.
    pub fn frames(&mut self) -> Vec<Vec<u8>> {
        let rest = std::mem::take(&mut self.0);
        rest.split(|&b| b == 0xFF).map(<[u8]>::to_vec).collect()
    }

    /// The bytes up to the next `0xFF` that is not doubled, with the doubles undone.
    fn stream(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some((&byte, rest)) = self.0.split_first() {
            if byte == 0xFF {
                if rest.first() == Some(&0xFF) {
                    out.push(0xFF);
                    self.0 = &rest[1..];
                    continue;
                }
                break;
            }
            out.push(byte);
            self.0 = rest;
        }
        out
    }
}

/// The terminal a script makes, and what it keeps between calls.
pub struct Run {
    pub term: Terminal,
    copy: TextCopy,
    rows_out: Vec<ViewRow>,
    damage: Damage,
    pins: Vec<wormhole_vt::Pin>,
    spans: Vec<wormhole_vt::Span>,
    handles: Vec<ImageHandle>,
    /// Whether the host takes the events after each step. Off, they pile up against the caps, the
    /// dropped history's backlog included.
    drain: bool,
    /// The history lines handed over so far, and the ones counted as skipped.
    archived: (usize, usize),
}

impl Run {
    /// The terminal the first bytes describe.
    pub fn new(bytes: &mut Bytes<'_>) -> Run {
        let cols = 1 + usize::from(bytes.byte()) % 120;
        let rows = 1 + usize::from(bytes.byte()) % 40;
        let flags = bytes.byte();
        let scrollback = [0, 3, 40, 2000][usize::from(flags & 3)];
        let mut policy = Policy::default();
        policy.screen_checksum = flags & 4 != 0;
        policy.window_ops = flags & 8 != 0;
        policy.title_report = flags & 16 != 0;
        // The second byte: whether the history that leaves is handed over, with a small backlog now
        // and then; lowered picture and string caps; a host that does not take its events.
        let knobs = bytes.byte();
        let mut limits = Limits::default();
        if knobs & 2 != 0 {
            limits.archive_backlog = 1 << (8 + usize::from(knobs >> 4));
        }
        if knobs & 4 != 0 {
            limits.image_bytes = 1 << (10 + usize::from(knobs >> 4));
            limits.image_count = usize::from(knobs >> 5);
            limits.osc = 16 << (knobs >> 4);
            limits.title = usize::from(knobs >> 3);
            limits.events = 1 + usize::from(knobs);
        }
        let mut builder = Terminal::builder()
            .size(cols, rows)
            .scrollback(scrollback)
            .policy(policy)
            .limits(limits)
            .archive_history(knobs & 1 != 0);
        // Bits 5 to 7 are the cell width when bit 4 is set; when it is not, bit 5 asks for strings
        // passed through (tiny limits, so that the oversize path runs as often as the delivery).
        if flags & 48 == 32 {
            let claims = Passthrough::new().osc(51234, 16).dcs(b"!", b'z', 16).apc(b'Q', 16);
            if let Ok(with) = builder.clone().passthrough(claims) {
                builder = with;
            }
        }
        let term = builder.build();
        let mut run = Run {
            term,
            copy: TextCopy::new(),
            rows_out: Vec::new(),
            damage: Damage::new(),
            pins: Vec::new(),
            spans: Vec::new(),
            handles: Vec::new(),
            drain: knobs & 8 == 0,
            archived: (0, 0),
        };
        if flags & 8 != 0 {
            run.feed(b"\x1b[?2027h");
        }
        if flags & 16 != 0 {
            run.term.set_cell_size(8 + u16::from(flags >> 5), 16);
        }
        run
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.term.feed(bytes);
        self.settle();
    }

    /// What a host does after every read: collect the events and replies, answer the pictures.
    fn settle(&mut self) {
        if !self.drain {
            self.term.clear_replies();
            return;
        }
        let mut answers = Vec::new();
        for event in self.term.events() {
            match event {
                Event::ImageData(upload) => answers.push(upload.image),
                Event::ImageFree(handle) => self.handles.retain(|&h| h != handle),
                Event::HistoryDropped(batch) => {
                    assert!(batch.lines.iter().all(|line| !line.cells.is_empty()), "an archived row has cells");
                    assert!(!batch.lines.is_empty() || batch.skipped > 0, "an empty batch");
                    self.archived.0 += batch.lines.len();
                    self.archived.1 += batch.skipped;
                }
                _ => {}
            }
        }
        for (n, handle) in answers.into_iter().enumerate() {
            // Most pictures decode; some fail the way a bad file does.
            let result = match n % 5 {
                3 => Err(wormhole_vt::image::ImageError::BadData),
                4 => Err(wormhole_vt::image::ImageError::TooBig),
                _ => Ok(()),
            };
            self.term.image_decoded(handle, result);
            self.handles.push(handle);
        }
        self.term.clear_replies();
    }

    fn point(&self, bytes: &mut Bytes<'_>) -> Point {
        let (a, b) = (bytes.byte(), bytes.byte());
        let history = self.term.history_len() as i32;
        let line = match a & 7 {
            // Mostly on the screen, sometimes in the history, now and then beyond either end.
            0 => -history - 1 - i32::from(b),
            1 => self.term.rows() as i32 + i32::from(b),
            2 => i32::from(a >> 3).wrapping_mul(0x0101_0101).wrapping_sub(0x4000_0000),
            3 | 4 => -(i32::from(b) % (history + 1)),
            _ => i32::from(b) % self.term.rows() as i32,
        };
        let column = match a >> 6 {
            0 => usize::from(b) % self.term.cols(),
            1 => self.term.cols() + usize::from(b),
            2 => usize::from(b) * usize::from(a),
            _ => usize::MAX - usize::from(b),
        };
        Point::new(Line(line), Column(column))
    }

    /// One host call read from `bytes`.
    pub fn call_with(&mut self, bytes: &[u8]) {
        self.call(&mut Bytes(bytes));
    }

    /// One host call.
    fn call(&mut self, bytes: &mut Bytes<'_>) {
        let op = bytes.byte();
        match op % 25 {
            0 => {
                let (cols, rows) = (1 + usize::from(bytes.byte()) % 150, 1 + usize::from(bytes.byte()) % 50);
                self.term.resize(cols, rows);
            }
            1 => {
                let scroll = match bytes.byte() % 4 {
                    0 => Scroll::Top,
                    1 => Scroll::Bottom,
                    _ => Scroll::Delta(i32::from(bytes.byte() as i8) * 3),
                };
                self.term.scroll(scroll);
            }
            2 => {
                let kind = [
                    SelectionType::Simple,
                    SelectionType::Block,
                    SelectionType::Semantic,
                    SelectionType::Lines,
                ][usize::from(bytes.byte()) % 4];
                let point = self.point(bytes);
                let side = if bytes.byte() & 1 == 0 { Side::Left } else { Side::Right };
                self.term.start_selection(kind, point, side);
            }
            3 => {
                let point = self.point(bytes);
                let side = if bytes.byte() & 1 == 0 { Side::Left } else { Side::Right };
                self.term.update_selection(point, side);
            }
            4 => {
                let _ = (self.term.selection_range(), self.term.selection_text());
                self.term.drop_empty_selection();
            }
            5 => {
                self.term.take_selection();
            }
            6 => {
                let from = self.point(bytes).line;
                let rows = usize::from(bytes.byte()) * 3;
                if let Some(next) = self.term.copy_text(from, rows, &mut self.copy) {
                    let _ = next;
                }
                // What a search does with the copy: map matches back to cells and pin them.
                let mut found = Vec::new();
                for line in self.copy.lines() {
                    let len = line.text().len();
                    if len > 0 {
                        let (a, b) = (usize::from(bytes.byte()) % len, usize::from(bytes.byte()) % len);
                        found.push(line.cells(a.min(b)..a.max(b) + 1));
                    }
                }
                for (start, end) in found.into_iter().flatten() {
                    let span = self.term.pin_span(start, end);
                    self.spans.push(span);
                }
            }
            7 => {
                self.term.copy_viewport(&mut self.rows_out, bytes.byte() & 1 == 0);
                self.term.take_damage(&mut self.damage);
                assert!(self.damage.rows().all(|row| row < self.term.rows()));
            }
            8 => {
                let (a, b) = (self.point(bytes), self.point(bytes));
                let shape = if bytes.byte() & 1 == 0 { TextShape::Stream } else { TextShape::Block };
                let _ = self.term.text_between(a, b, shape);
                let _ = self.term.link_span(a);
                let _ = self.term.cell(b);
                let _ = self.term.grid().word_start(a, |c| c == ' ');
                let _ = self.term.grid().word_end(b, |c| c == ' ');
                let _ = self.term.grid().bracket_match(a);
                let _ = self.term.grid().line_extent(a.line);
            }
            9 => {
                let point = self.point(bytes);
                let pin = self.term.pin(point);
                self.pins.push(pin);
            }
            10 => {
                if let Some(pin) = self.pins.pop() {
                    let _ = self.term.pin_point(pin);
                    if bytes.byte() & 1 == 0 {
                        let point = self.point(bytes);
                        self.term.move_pin(pin, point);
                        self.pins.insert(0, pin);
                    } else {
                        self.term.unpin(pin);
                    }
                }
            }
            11 => {
                for span in std::mem::take(&mut self.spans) {
                    let _ = (self.term.span_points(span), self.term.span_holds(span, "a"));
                    if bytes.byte() & 1 == 0 {
                        self.term.unpin_span(span);
                    }
                }
            }
            12 => self.term.clear_history(),
            13 => {
                let lines = [0, 1, 5, 100, 5000][usize::from(bytes.byte()) % 5];
                self.term.set_scrollback_lines(lines);
            }
            14 => {
                let _ = (self.term.screen_text(), self.term.restore_lines(usize::from(bytes.byte())));
            }
            15 => {
                let line = self.point(bytes).line;
                let (mut cells, mut extras): (Vec<Cell>, RowExtras) = (Vec::new(), RowExtras::default());
                let _ = self.term.copy_row(line, &mut cells, Some(&mut extras));
                let _ = (self.term.row_text(line, bytes.byte() & 1 == 0), self.term.is_wrapped(line));
                let _ = self.term.row_mark(line);
                let block = u32::from(bytes.byte());
                let content = [BlockPart::None, BlockPart::Prompt, BlockPart::Command, BlockPart::Output]
                    [usize::from(bytes.byte()) % 4];
                let _ = self.term.block_text(line, block, content);
            }
            16 => {
                let mut placements = Vec::new();
                self.term.placements(&mut placements);
                let _ = (self.term.has_placements(), self.term.cursor(), self.term.mode(), self.term.modes());
            }
            17 => self.feed(b"\x1b[?2027h"),
            18 => self.feed(b"\x1b[?2027l"),
            19 => {
                // The host answers a handle it was never given, or one twice.
                let handle = ImageHandle::from_raw(u64::from(bytes.byte()));
                self.term.image_decoded(handle, Ok(()));
                if let Some(&known) = self.handles.first() {
                    self.term.image_decoded(known, Err(wormhole_vt::image::ImageError::BadData));
                }
            }
            20 => {
                let mut policy = self.term.policy();
                policy.screen_checksum = bytes.byte() % 2 == 0;
                self.term.set_policy(policy);
                self.term.set_word_separators(if bytes.byte() & 1 == 0 { " " } else { "" });
            }
            22 => {
                let on = bytes.byte() & 1 == 0;
                self.term.set_archive_history(on);
                assert_eq!(self.term.archive_history(), on);
            }
            23 => self.drain = bytes.byte() & 1 == 0,
            24 => {
                let mut limits = Limits::default();
                let (a, b) = (bytes.byte(), bytes.byte());
                limits.image_bytes = u64::from(a) << (b % 24);
                limits.image_count = usize::from(a % 8);
                limits.archive_backlog = usize::from(b) << (a % 16);
                limits.osc = usize::from(a) * 4;
                limits.events = usize::from(b);
                limits.replies = usize::from(a) << (b % 8);
                self.term.set_limits(limits);
                assert!(self.term.limits().image_bytes <= Limits::default().image_bytes);
            }
            _ => {
                self.term.set_cell_size(u16::from(bytes.byte()), u16::from(bytes.byte()));
                let _ = self.term.title();
            }
        }
        self.settle();
    }

    /// Check what must always hold.
    pub fn check(&self, what: &str) {
        if let Err(why) = self.term.check_invariants() {
            panic!("{what}: {why}");
        }
        let cursor = self.term.cursor();
        assert!(cursor.point.line.0 >= 0 && (cursor.point.line.0 as usize) < self.term.rows());
        assert!(cursor.point.column.0 < self.term.cols());
    }

    /// Run the script to its end, checking after every step.
    pub fn script(bytes: &mut Bytes<'_>) -> Run {
        let mut run = Run::new(bytes);
        run.check("at the start");
        let mut steps = 0;
        while !bytes.is_empty() && steps < 400 {
            let stream = bytes.stream();
            if !stream.is_empty() {
                run.feed(&stream);
                run.check("after text");
            }
            if !bytes.is_empty() {
                bytes.byte();
                run.call(bytes);
                run.check("after a call");
            }
            steps += 1;
        }
        run
    }
}

/// Base64 (RFC 4648, padded), to turn bytes into what a picture's payload carries.
pub mod base64 {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            let n = chunk.iter().enumerate().fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
}
