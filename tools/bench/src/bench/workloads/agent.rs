//! The two tests that redraw the way a coding agent's terminal UI does. Such a program has two
//! renderers; each frame of either is one synchronized update (mode 2026) and one write.
//!
//! - `agentredraw`, the classic renderer: it draws on the main screen, the conversation goes into the
//!   terminal's own scrollback, and a live region at the bottom (the reply being streamed, a status
//!   line, an input box and a hint line) is redrawn in place, all of it, every frame.
//! - `agentscreen`, the fullscreen renderer: the alternate screen, the input box pinned at the
//!   bottom, and only what changed between two frames is sent.
//!
//! Both show the same conversation: replies made of the blocks `agentstream` writes (paragraphs,
//! bullets, code, a diff), arriving a few words at a time under the status line `agentstream` draws.

use std::collections::VecDeque;

use super::realistic::{CODE, CODE_BG, PROSE, choose, cut, highlight, prose, spinner};
use crate::canvas::{self, Rng};
use crate::term::Size;

/// The fewest frames either test sends, so there is a rate to measure.
const MIN_FRAMES: usize = 30;
/// The most. Frames are small, so `target` bytes alone would be thousands of them, more than a
/// terminal that draws each one can take in a minute.
const REDRAW_MAX_FRAMES: usize = 1200;
const SCREEN_MAX_FRAMES: usize = 2400;
/// How often the classic renderer repaints everything (see `agentredraw`).
const REPAINT_EVERY: usize = 200;
/// How many rows of the streaming reply the live region shows: the tail of it.
const LIVE_TAIL: usize = 14;
/// Cells that may lie unchanged between two changed ones and still go out in one run: a cursor move
/// costs more than a few cells sent again.
const JOIN: usize = 3;

const BORDER: &str = "\x1b[38;2;100;106;120m";
const PLACEHOLDER: &str = "Try \"fix the failing test\"";
const HINT: &str = "  ? for shortcuts   esc to interrupt";

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
}

// --- text -------------------------------------------------------------------------------------

/// Text with its colors, and the cells it takes: a word as a model streams it (with the space after
/// it), or a whole row.
#[derive(Clone, Default)]
struct Row {
    bytes: Vec<u8>,
    cells: usize,
}

impl Row {
    fn new(bytes: Vec<u8>) -> Self {
        let cells = cells(&bytes).len();
        Self { bytes, cells }
    }
}

/// One cell: its character and the color sequences in force on it.
type Cell = (char, Vec<u8>);

/// The cells `bytes` draws: its characters with the colors set on them. Only SGR sequences are
/// read; other sequences and CRs take no cell.
fn cells(bytes: &[u8]) -> Vec<Cell> {
    let text: Vec<char> = String::from_utf8_lossy(bytes).chars().collect();
    let (mut cells, mut style, mut i) = (Vec::new(), Vec::new(), 0);
    while i < text.len() {
        match text[i] {
            '\x1b' if text.get(i + 1) == Some(&'[') => {
                let Some(p) = text[i + 2..].iter().position(|c| ('@'..='~').contains(c)) else { break };
                let end = i + 2 + p;
                if text[end] == 'm' {
                    let seq: String = text[i..=end].iter().collect();
                    if seq == "\x1b[0m" || seq == "\x1b[m" {
                        style.clear();
                    } else {
                        style.extend_from_slice(seq.as_bytes());
                    }
                }
                i = end;
            }
            '\r' => {}
            c => cells.push((c, style.clone())),
        }
        i += 1;
    }
    cells
}

/// What a model streams: a piece of a line, or the end of one.
enum Tok {
    Text(Row),
    Break,
}

/// Hands out up to `n` pieces of text: they go on the last row, and a break starts a new one.
/// `produced` counts the cells handed out.
fn reveal(toks: &mut VecDeque<Tok>, rows: &mut Vec<Row>, n: usize, produced: &mut usize) {
    let mut left = n;
    while left > 0 {
        match toks.pop_front() {
            Some(Tok::Text(piece)) => {
                left -= 1;
                *produced += piece.cells;
                match rows.last_mut() {
                    Some(row) => {
                        row.bytes.extend_from_slice(&piece.bytes);
                        row.cells += piece.cells;
                    }
                    None => rows.push(piece),
                }
            }
            Some(Tok::Break) => rows.push(Row::default()),
            None => break,
        }
    }
}

/// Everything in `toks` as rows, without the empty one the last break opens.
fn rows_of(mut toks: VecDeque<Tok>) -> Vec<Row> {
    let mut rows = vec![Row::default()];
    reveal(&mut toks, &mut rows, usize::MAX, &mut 0);
    rows.pop();
    rows
}

/// The rows of a reply, without the empty ones the stream leaves at its end.
fn trimmed(rows: &[Row]) -> &[Row] {
    &rows[..rows.iter().rposition(|r| r.cells > 0).map_or(0, |i| i + 1)]
}

/// A line of text cut into words: each with the space after it and the colors it sets and resets.
fn pieces(line: &[u8]) -> Vec<Row> {
    line.split_inclusive(|b| *b == b' ').map(|w| Row::new(w.to_vec())).collect()
}

/// `words` as lines of at most `width` cells, the first starting with `lead` and the others indented
/// under it, then the end of the last line.
fn wrap(toks: &mut VecDeque<Tok>, lead: Row, words: Vec<Row>, width: usize) {
    let indent = Row { bytes: vec![b' '; lead.cells], cells: lead.cells };
    let mut used = lead.cells;
    let put = |toks: &mut VecDeque<Tok>, piece: Row| {
        if !piece.bytes.is_empty() {
            toks.push_back(Tok::Text(piece));
        }
    };
    put(toks, lead);
    for word in words {
        if used + word.cells > width && used > indent.cells {
            toks.push_back(Tok::Break);
            put(toks, indent.clone());
            used = indent.cells;
        }
        used += word.cells;
        put(toks, word);
    }
    toks.push_back(Tok::Break);
}

/// One block of an answer: paragraphs, a heading, bullets, a fenced code block or a diff, the mix
/// `agentstream` writes. Code and diff rows are padded with their background to `width`, so every
/// row is at most that wide.
fn block(rng: &mut Rng, width: usize, toks: &mut VecDeque<Tok>) {
    let line = |toks: &mut VecDeque<Tok>, bytes: Vec<u8>| {
        toks.push_back(Tok::Text(Row::new(bytes)));
        toks.push_back(Tok::Break);
    };
    // A source line on its background, filled to `width`.
    let source = |rng: &mut Rng, bg: &str, mark: &str, room: usize| {
        let src = cut(CODE[rng.below(CODE.len())], room);
        let mut bytes = [bg, mark].concat().into_bytes();
        highlight(&mut bytes, src);
        if !bg.is_empty() {
            bytes.resize(bytes.len() + room - src.len(), b' ');
        }
        bytes.extend_from_slice(b"\x1b[0m");
        bytes
    };
    match rng.below(10) {
        0..=3 => {
            let mut text = Vec::new();
            let lines = 2 + rng.below(3);
            prose(rng, &mut text, lines * width);
            wrap(toks, Row::default(), pieces(&text), width);
            toks.push_back(Tok::Break);
        }
        4 => {
            let mut bytes = b"\x1b[1m\x1b[38;2;130;180;255m## ".to_vec();
            let mut used = 3;
            for _ in 0..2 + rng.below(3) {
                let word = choose(rng, &PROSE);
                if used + word.len() + 1 > width {
                    break;
                }
                push_str(&mut bytes, word);
                bytes.push(b' ');
                used += word.len() + 1;
            }
            bytes.extend_from_slice(b"\x1b[0m");
            line(toks, bytes);
        }
        5 | 6 => {
            for _ in 0..3 + rng.below(3) {
                let mut text = Vec::new();
                let lines = 1 + rng.below(2);
                prose(rng, &mut text, width.saturating_sub(6) * lines);
                let bullet = Row::new("  \x1b[38;2;130;180;255m•\x1b[39m ".as_bytes().to_vec());
                wrap(toks, bullet, pieces(&text), width);
            }
            toks.push_back(Tok::Break);
        }
        7 | 8 => {
            line(toks, b"\x1b[2m```rust\x1b[0m".to_vec());
            for _ in 0..4 + rng.below(7) {
                line(toks, source(rng, CODE_BG, "", width));
            }
            line(toks, b"\x1b[2m```\x1b[0m".to_vec());
        }
        _ => {
            line(toks, b"\x1b[2m--- a/src/session.rs\x1b[0m".to_vec());
            let hunk = format!("\x1b[36m@@ -{},7 +{},9 @@\x1b[0m", 10 + rng.below(300), 10 + rng.below(300));
            line(toks, hunk.into_bytes());
            for _ in 0..5 + rng.below(6) {
                let (mark, bg) = match rng.below(3) {
                    0 => ("-", "\x1b[48;2;70;30;35m"),
                    1 => ("+", "\x1b[48;2;25;60;35m"),
                    _ => (" ", ""),
                };
                line(toks, source(rng, bg, mark, width - 1));
            }
        }
    }
}

/// A whole reply: one to three blocks.
fn answer(rng: &mut Rng, width: usize, toks: &mut VecDeque<Tok>) {
    for _ in 0..1 + rng.below(3) {
        block(rng, width, toks);
    }
}

/// What the user typed, as the transcript keeps it: a dim `>`, a line or two of words and a blank row.
fn prompt(rng: &mut Rng, width: usize) -> VecDeque<Tok> {
    let mut text = Vec::new();
    let want = 20 + rng.below(width);
    prose(rng, &mut text, want);
    let mut toks = VecDeque::new();
    wrap(&mut toks, Row::new(b"\x1b[2m>\x1b[22m ".to_vec()), pieces(&text), width);
    toks.push_back(Tok::Break);
    toks
}

// --- the screen's furniture -------------------------------------------------------------------

/// A rule of the input box: `left`, a line and `right`, `w` cells wide.
fn rule(w: usize, left: &str, right: &str) -> Row {
    Row::new(format!("{BORDER}{left}{}{right}\x1b[0m", "─".repeat(w.saturating_sub(2))).into_bytes())
}

/// The input box's text row, `w` cells wide: the typed text, or a dim hint at what to type.
fn input_row(w: usize, typed: &str) -> Row {
    let room = w.saturating_sub(5);
    let text = cut(if typed.is_empty() { PLACEHOLDER } else { typed }, room);
    let styled = if typed.is_empty() { format!("\x1b[2m{text}\x1b[0m") } else { text.to_string() };
    let pad = " ".repeat(room - text.len());
    Row::new(format!("{BORDER}│\x1b[0m > {styled}{pad}{BORDER}│\x1b[0m").into_bytes())
}

fn hint_row(w: usize) -> Row {
    Row::new(format!("\x1b[2m{}\x1b[0m", cut(HINT, w)).into_bytes())
}

/// Closes a frame: its size is what `out` grew by since `mark`, and it goes out as one write.
fn end_frame(out: &[u8], sizes: &mut Vec<usize>, mark: &mut usize) {
    sizes.push(out.len() - *mark);
    *mark = out.len();
}

/// The last bytes of a payload belong to its last frame's write.
fn finish(out: &mut Vec<u8>, sizes: &mut [usize], mark: usize) {
    push_str(out, "\x1b[0m");
    if let Some(last) = sizes.last_mut() {
        *last += out.len() - mark;
    }
}

// --- agentredraw ------------------------------------------------------------------------------

/// Each row of `rows` followed by a line break.
fn print_rows(out: &mut Vec<u8>, rows: &[Row]) {
    for row in rows {
        out.extend_from_slice(&row.bytes);
        push_str(out, "\r\n");
    }
}

/// The live region's rows, top to bottom: the tail of the streaming reply, a blank row, the status
/// line, the input box and the hint line. Each starts by erasing what was on its row.
fn live_rows(reply: &[Row], w: usize, tail: usize, tick: usize, produced: usize) -> Vec<Vec<u8>> {
    let erase = |row: &[u8]| [&b"\x1b[2K"[..], row].concat();
    let mut rows: Vec<Vec<u8>> =
        reply[reply.len().saturating_sub(tail)..].iter().map(|r| erase(&r.bytes)).collect();
    rows.push(erase(b""));
    // The status line brings its own CR and EL.
    let mut status = Vec::new();
    spinner(&mut status, tick, produced);
    rows.push(status);
    rows.push(erase(&rule(w, "╭", "╮").bytes));
    rows.push(erase(&input_row(w, "").bytes));
    rows.push(erase(&rule(w, "╰", "╯").bytes));
    rows.push(erase(&hint_row(w).bytes));
    rows
}

/// An agent's classic renderer. The live region (6 to 20 rows: the reply's last rows, a blank row,
/// the status line, the input box and the hint) is redrawn in place every frame: back up to its top
/// (CR, CUU), then each row erased and written again, so all of it is sent however little changed.
/// A few words arrive per frame and the status line ticks.
///
/// A finished reply is committed: the region is erased, the reply (and the next prompt) printed
/// once, so it scrolls into the scrollback, and the region starts again below. Every `REPAINT_EVERY`
/// frames the whole transcript is repainted (home, erase the screen and the scrollback, every
/// committed row, the region): this models what a classic renderer may do after a resize, to draw
/// the scrollback anew. Those frames grow with the transcript, like a long session's.
///
/// Returns the size of each frame, which goes out as one write.
pub(super) fn agentredraw(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) -> Vec<usize> {
    let w = (size.cols as usize).clamp(20, 100);
    let width = w - 4;
    // The region has to fit the screen, or its top is out of reach.
    let tail = LIVE_TAIL.min((size.rows as usize).saturating_sub(8)).max(1);
    let (mut sizes, mut mark) = (Vec::new(), out.len());
    push_str(out, "\x1b[H\x1b[2J");
    let mut transcript: Vec<Row> = Vec::new();
    let mut reply: Vec<Row> = Vec::new();
    let mut toks = VecDeque::new();
    let (mut tick, mut produced, mut shown) = (0, 0, 0);
    while sizes.len() < MIN_FRAMES || (out.len() < target && sizes.len() < REDRAW_MAX_FRAMES) {
        tick += 1;
        let first = transcript.len();
        if toks.is_empty() {
            let done = trimmed(&reply);
            if !done.is_empty() {
                transcript.extend_from_slice(done);
                transcript.push(Row::default());
            }
            transcript.extend(rows_of(prompt(rng, width)));
            reply = vec![Row::default()];
            answer(rng, width, &mut toks);
        } else {
            reveal(&mut toks, &mut reply, rng.below(5), &mut produced);
        }
        let live = live_rows(trimmed(&reply), w, tail, tick, produced);

        push_str(out, "\x1b[?2026h");
        if tick % REPAINT_EVERY == 0 {
            push_str(out, "\x1b[H\x1b[2J\x1b[3J");
            print_rows(out, &transcript);
        } else {
            out.push(b'\r');
            if shown > 1 {
                push_str(out, &format!("\x1b[{}A", shown - 1));
            }
            if transcript.len() > first {
                push_str(out, "\x1b[J");
                print_rows(out, &transcript[first..]);
            }
        }
        out.extend_from_slice(&live.join(&b"\r\n"[..]));
        shown = live.len();
        push_str(out, "\x1b[?2026l");
        end_frame(out, &mut sizes, &mut mark);
    }
    finish(out, &mut sizes, mark);
    sizes
}

// --- agentscreen ------------------------------------------------------------------------------

/// Brings screen row `y` from `old` to `new` the way a diffing renderer does when it compares rows:
/// nothing when they are the same, only the new end when text was added to it, otherwise the whole
/// row (and an erase when the old one was longer).
fn diff_row(out: &mut Vec<u8>, y: usize, old: &Row, new: &Row) {
    if new.bytes == old.bytes {
        return;
    }
    if new.bytes.starts_with(&old.bytes) {
        canvas::goto(out, old.cells as u16, y as u16);
        out.extend_from_slice(&new.bytes[old.bytes.len()..]);
    } else {
        canvas::goto(out, 0, y as u16);
        out.extend_from_slice(&new.bytes);
        if new.cells < old.cells {
            push_str(out, "\x1b[K");
        }
    }
}

/// Brings screen row `y` from `old` to `new` cell by cell: each run of changed cells at its place
/// with the colors it needs (runs a few cells apart are joined), and an erase when the row got
/// shorter.
fn diff_cells(out: &mut Vec<u8>, y: usize, old: &[Cell], new: &[Cell]) {
    let changed = |i: usize| old.get(i) != Some(&new[i]);
    let (mut x, mut drew) = (0, false);
    while x < new.len() {
        if !changed(x) {
            x += 1;
            continue;
        }
        let start = x;
        let mut end = start + 1;
        let mut j = end;
        while j < new.len() && j <= end + JOIN {
            if changed(j) {
                end = j + 1;
            }
            j += 1;
        }
        canvas::goto(out, start as u16, y as u16);
        let mut style: Option<&Vec<u8>> = None;
        for (c, s) in &new[start..end] {
            if style != Some(s) {
                push_str(out, "\x1b[0m");
                out.extend_from_slice(s);
                style = Some(s);
            }
            out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
        }
        drew = true;
        x = end;
    }
    if drew {
        push_str(out, "\x1b[0m");
    }
    if old.len() > new.len() {
        canvas::goto(out, new.len() as u16, y as u16);
        push_str(out, "\x1b[K");
    }
}

/// An agent's fullscreen renderer: the alternate screen, the messages above, then the status line
/// and the input box with its hint pinned at the bottom. Between two frames only what changed is
/// sent: the status line's glyph, seconds and tokens, the words added to the end of the reply, now
/// and then a character typed into the box, and the cursor parked where the next one goes.
///
/// When the reply grows past the bottom of the message area the area scrolls, and it does so the way
/// a renderer that compares frames does without help from the terminal: every message row whose
/// text moved is written again (no scroll region and no SU, so nothing is left to reset).
///
/// Returns the size of each frame, which goes out as one write.
pub(super) fn agentscreen(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) -> Vec<usize> {
    let w = (size.cols as usize).clamp(20, 100);
    let width = w - 4;
    // From the bottom: the hint, the input box (three rows), the status line; the messages above.
    let area = (size.rows as usize).saturating_sub(5).max(1);
    let (status_y, input_y) = (area, area + 2);
    let (mut sizes, mut mark) = (Vec::new(), out.len());
    // The benchmark is already on the alternate screen: start the program as a shell would, leaving
    // it for the shell's screen and entering one afresh. The stream ends on it, where it began.
    push_str(out, "\x1b[?1049l\x1b[?1049h\x1b[2J");

    // A conversation that already fills the area, so the first frames scroll as the later ones do.
    let (mut convo, mut toks, mut produced, mut spent) = (vec![Row::default()], VecDeque::new(), 0, 0);
    while convo.len() <= area {
        reveal(&mut prompt(rng, width), &mut convo, usize::MAX, &mut spent);
        answer(rng, width, &mut toks);
        reveal(&mut toks, &mut convo, usize::MAX, &mut spent);
    }
    let mut shown = vec![Row::default(); area];
    let (mut status_shown, mut input_shown) = (Vec::new(), Vec::new());
    let (mut typed, mut cursor, mut tick) = (String::new(), 0, 0);
    while sizes.len() < MIN_FRAMES || (out.len() < target && sizes.len() < SCREEN_MAX_FRAMES) {
        tick += 1;
        if toks.is_empty() {
            reveal(&mut prompt(rng, width), &mut convo, usize::MAX, &mut spent);
            answer(rng, width, &mut toks);
        } else {
            reveal(&mut toks, &mut convo, rng.below(5), &mut produced);
        }

        push_str(out, "\x1b[?2026h");
        // The messages are compared row by row, the status line and the input box cell by cell.
        let view = &convo[convo.len() - area..];
        for (y, row) in view.iter().enumerate() {
            diff_row(out, y, &shown[y], row);
        }
        shown.clone_from_slice(view);
        let mut status = Vec::new();
        spinner(&mut status, tick, produced);
        let status = cells(&status);
        diff_cells(out, status_y, &status_shown, &status);
        status_shown = status;
        // Now and then a character is typed at the end of the box (and sent when the box is full), or
        // the cursor moves inside the text.
        let mut typing = tick == 1;
        match rng.below(30) {
            0 | 1 => {
                if typed.len() >= (w - 6).min(30) {
                    typed.clear();
                } else {
                    typed.push(if rng.below(6) == 0 { ' ' } else { (b'a' + rng.below(26) as u8) as char });
                }
                cursor = typed.len();
                typing = true;
            }
            2 => cursor = rng.below(typed.len() + 1),
            _ => {}
        }
        if typing {
            let input = cells(&input_row(w, &typed).bytes);
            diff_cells(out, input_y, &input_shown, &input);
            input_shown = input;
        }
        // The box's borders and the hint never change: only the first frame draws them.
        if tick == 1 {
            let furniture =
                [(area + 1, rule(w, "╭", "╮")), (area + 3, rule(w, "╰", "╯")), (area + 4, hint_row(w))];
            for (y, row) in furniture {
                diff_row(out, y, &Row::default(), &row);
            }
        }
        canvas::goto(out, (4 + cursor) as u16, input_y as u16);
        push_str(out, "\x1b[?2026l");
        end_frame(out, &mut sizes, &mut mark);
    }
    finish(out, &mut sizes, mark);
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size { cols: 100, rows: 30, px_w: 0, px_h: 0 };

    fn redraw(seed: u64, target: usize) -> (String, Vec<usize>) {
        let mut out = Vec::new();
        let sizes = agentredraw(&mut Rng::new(seed), &mut out, target, SIZE);
        (String::from_utf8_lossy(&out).into_owned(), sizes)
    }

    fn screen(seed: u64, target: usize) -> (String, Vec<usize>) {
        let mut out = Vec::new();
        let sizes = agentscreen(&mut Rng::new(seed), &mut out, target, SIZE);
        (String::from_utf8_lossy(&out).into_owned(), sizes)
    }

    /// What is inside each synchronized update of a payload.
    fn updates(s: &str) -> Vec<&str> {
        s.split("\x1b[?2026h").skip(1).map(|u| u.split("\x1b[?2026l").next().unwrap_or("")).collect()
    }

    /// Every CSI sequence in `s`: its parameters and its final character.
    fn csis(s: &str) -> Vec<(String, char)> {
        let mut found = Vec::new();
        let mut rest = s;
        while let Some(i) = rest.find("\x1b[") {
            rest = &rest[i + 2..];
            let Some(end) = rest.find(|c: char| ('@'..='~').contains(&c)) else { break };
            found.push((rest[..end].to_string(), rest[end..].chars().next().unwrap_or(' ')));
            rest = &rest[end..];
        }
        found
    }

    #[test]
    fn a_row_counts_cells_and_remembers_colors() {
        let row = Row::new("\x1b[1mhé\x1b[22m x\x1b[0m\r".as_bytes().to_vec());
        assert_eq!(row.cells, 4);
        let found = cells(&row.bytes);
        assert_eq!(found[0], ('h', b"\x1b[1m".to_vec()));
        assert_eq!(found[3], ('x', b"\x1b[1m\x1b[22m".to_vec()));
    }

    #[test]
    fn replies_fit_the_width_they_are_given() {
        for width in [40usize, 96] {
            let mut rng = Rng::new(width as u64);
            let mut rows = vec![Row::default()];
            let mut toks = VecDeque::new();
            for _ in 0..40 {
                answer(&mut rng, width, &mut toks);
            }
            reveal(&mut toks, &mut rows, usize::MAX, &mut 0);
            assert!(rows.len() > 100);
            let widest = rows.iter().map(|r| r.cells).max().unwrap_or(0);
            assert!(widest <= width, "{width}: {widest}");
            // Every kind of block `agentstream` writes is there.
            let text: String = rows.iter().map(|r| String::from_utf8_lossy(&r.bytes).into_owned()).collect();
            for needle in
                ["•", "```rust", "@@ -", "## ", "\x1b[1m", "\x1b[48;2;40;44;52m", "\x1b[48;2;25;60;35m"]
            {
                assert!(text.contains(needle), "{width}: no {needle:?}");
            }
        }
    }

    #[test]
    fn frames_are_whole_synchronized_writes() {
        for (s, sizes) in [redraw(1, 200_000), screen(1, 200_000)] {
            assert_eq!(sizes.iter().sum::<usize>(), s.len());
            assert_eq!(s.matches("\x1b[?2026h").count(), sizes.len());
            assert_eq!(s.matches("\x1b[?2026l").count(), sizes.len());
            // Each write holds one update, and the last bytes of the payload ride with the last.
            let mut at = 0;
            for n in &sizes {
                let write = s.get(at..at + n).unwrap_or("");
                assert_eq!(
                    (write.matches("\x1b[?2026h").count(), write.matches("\x1b[?2026l").count()),
                    (1, 1)
                );
                at += n;
            }
            assert!(s.ends_with("\x1b[0m"));
        }
    }

    #[test]
    fn both_stop_at_their_frame_limits() {
        assert_eq!(redraw(5, 1).1.len(), MIN_FRAMES);
        assert_eq!(redraw(5, usize::MAX).1.len(), REDRAW_MAX_FRAMES);
        assert_eq!(screen(5, 1).1.len(), MIN_FRAMES);
        assert_eq!(screen(5, usize::MAX).1.len(), SCREEN_MAX_FRAMES);
    }

    #[test]
    fn the_same_seed_gives_the_same_frames() {
        assert_eq!(redraw(7, 100_000), redraw(7, 100_000));
        assert_ne!(redraw(7, 100_000).0, redraw(8, 100_000).0);
        assert_eq!(screen(7, 100_000), screen(7, 100_000));
        assert_ne!(screen(7, 100_000).0, screen(8, 100_000).0);
    }

    #[test]
    fn the_classic_renderer_stays_on_the_main_screen() {
        let (s, _) = redraw(2, 300_000);
        assert!(!s.contains("\x1b[?1049"));
        assert!(csis(&s).iter().all(|(_, last)| !matches!(last, 'r' | 'S' | 'T')));
    }

    #[test]
    fn the_live_region_is_redrawn_over_the_rows_it_took() {
        let (s, _) = redraw(1, usize::MAX);
        let mut before = 0;
        let mut heights = Vec::new();
        for (i, frame) in updates(&s).into_iter().enumerate() {
            // Every row of the region starts with an erase.
            let rows = frame.matches("\x1b[2K").count();
            if i > 0 && !frame.contains("\x1b[3J") {
                let up = format!("\r\x1b[{}A", before - 1);
                assert!(frame.starts_with(&up), "frame {i} after {before} rows: {:?}", frame.get(..24));
            }
            heights.push(rows);
            before = rows;
        }
        // It starts as the status line and the box, and grows with the reply up to a tail of it.
        assert_eq!((heights.iter().min(), heights.iter().max()), (Some(&6), Some(&(6 + LIVE_TAIL))));
    }

    #[test]
    fn finished_replies_are_committed_above_the_region() {
        let (s, _) = redraw(2, usize::MAX);
        let commits: Vec<&str> = updates(&s).into_iter().filter(|u| u.contains("\x1b[J")).collect();
        assert!(commits.len() >= 20, "{}", commits.len());
        for frame in commits {
            // Erased from the top of the old region, then the rows once, then the region again.
            let printed = frame.split_once("\x1b[J").map_or("", |(_, rest)| rest);
            let rows = printed.split("\x1b[2K").next().unwrap_or("");
            assert!(rows.ends_with("\r\n") && rows.contains("\x1b[2m>"), "{rows:?}");
        }
    }

    #[test]
    fn the_whole_transcript_is_repainted_on_schedule_and_grows() {
        let (s, sizes) = redraw(3, usize::MAX);
        let frames = updates(&s);
        let repaints: Vec<usize> =
            (0..frames.len()).filter(|i| frames[*i].starts_with("\x1b[H\x1b[2J\x1b[3J")).collect();
        let due: Vec<usize> = (1..=frames.len() / REPAINT_EVERY).map(|n| n * REPAINT_EVERY - 1).collect();
        assert!(due.len() >= 5);
        assert_eq!(repaints, due);
        let lens: Vec<usize> = repaints.iter().map(|i| sizes[*i]).collect();
        assert!(lens.windows(2).all(|pair| pair[0] < pair[1]), "{lens:?}");
        assert!(lens[lens.len() - 1] > 2 * lens[0], "{lens:?}");
        // A repaint draws the region again after the transcript, like any other frame.
        assert!(repaints.iter().all(|i| frames[*i].contains("? for shortcuts")));
    }

    #[test]
    fn no_redrawn_row_is_wider_than_the_screen() {
        let (s, _) = redraw(4, 300_000);
        let rows = s.replace("\x1b[?2026l", "\r\n");
        let widest = rows.split("\r\n").map(|row| cells(row.as_bytes()).len()).max().unwrap_or(0);
        assert!(widest <= SIZE.cols as usize, "{widest}");
    }

    #[test]
    fn the_fullscreen_renderer_enters_and_leaves_the_alternate_screen_evenly() {
        let (s, _) = screen(1, 300_000);
        // Started as a shell would, and ended on the alternate screen it began on.
        assert!(s.starts_with("\x1b[?1049l\x1b[?1049h"));
        assert_eq!(s.matches("\x1b[?1049h").count(), s.matches("\x1b[?1049l").count());
        assert!(s.rfind("\x1b[?1049h") > s.rfind("\x1b[?1049l"));
        // It scrolls by writing rows again: no scroll region to reset, no SU or SD.
        assert!(csis(&s).iter().all(|(_, last)| !matches!(last, 'r' | 'S' | 'T')));
    }

    #[test]
    fn the_fullscreen_renderer_never_draws_outside_the_screen() {
        let (s, _) = screen(2, usize::MAX);
        let (cols, rows) = (SIZE.cols as usize, SIZE.rows as usize);
        let mut chars = s.chars().peekable();
        let mut x = 0;
        while let Some(c) = chars.next() {
            match c {
                '\x1b' if chars.peek() == Some(&'[') => {
                    chars.next();
                    let mut params = String::new();
                    let last = loop {
                        match chars.next() {
                            Some(c) if ('@'..='~').contains(&c) => break c,
                            Some(c) => params.push(c),
                            None => break ' ',
                        }
                    };
                    if last == 'H' {
                        let (row, col) = params.split_once(';').unwrap_or(("0", "0"));
                        let (row, col) = (row.parse().unwrap_or(0), col.parse().unwrap_or(0));
                        assert!((1..=rows).contains(&row) && (1..=cols).contains(&col), "{params}");
                        x = col - 1;
                    }
                }
                '\r' | '\n' => panic!("every write of a frame is positioned: no line breaks"),
                c if c >= ' ' => {
                    x += 1;
                    assert!(x <= cols, "wrote past the last column");
                }
                _ => {}
            }
        }
    }

    #[test]
    fn the_fullscreen_renderer_sends_what_changed() {
        let (_, sizes) = screen(3, usize::MAX);
        let mut rest = sizes[1..].to_vec();
        rest.sort_unstable();
        let (median, p90) = (rest[rest.len() / 2], rest[rest.len() * 9 / 10]);
        // The first frame paints the screen, most of the others are a few cells, and the ones that
        // scroll write the moved rows again.
        assert!(median * 10 < sizes[0], "{} against {median}", sizes[0]);
        assert!(p90 > median * 10, "{p90} against {median}");
    }

    #[test]
    fn the_status_line_changes_by_its_glyph_alone() {
        let status = |tick| {
            let mut bytes = Vec::new();
            spinner(&mut bytes, tick, 40);
            cells(&bytes)
        };
        let mut out = Vec::new();
        diff_cells(&mut out, 4, &status(1), &status(2));
        assert_eq!(String::from_utf8_lossy(&out), "\x1b[5;1H\x1b[0m\x1b[38;2;215;119;87m✳\x1b[0m");
    }

    #[test]
    fn changed_cells_are_joined_when_close_and_a_shorter_row_is_erased() {
        let cells = |s: &str| cells(s.as_bytes());
        let mut out = Vec::new();
        diff_cells(&mut out, 0, &cells("same"), &cells("same"));
        assert!(out.is_empty());
        // `b` and `e` changed with `cd` between them: one run. The row lost its last cell.
        diff_cells(&mut out, 0, &cells("abcdefgh"), &cells("aXcdYfg"));
        assert_eq!(String::from_utf8_lossy(&out), "\x1b[1;2H\x1b[0mXcdY\x1b[0m\x1b[1;8H\x1b[K");
        out.clear();
        // Five cells apart is too far: two runs.
        diff_cells(&mut out, 0, &cells("a.....b"), &cells("X.....Y"));
        assert_eq!(String::from_utf8_lossy(&out), "\x1b[1;1H\x1b[0mX\x1b[1;7H\x1b[0mY\x1b[0m");
    }

    #[test]
    fn a_row_is_written_whole_or_only_at_its_new_end() {
        let row = |s: &str| Row::new(s.as_bytes().to_vec());
        let mut out = Vec::new();
        diff_row(&mut out, 2, &row("ab "), &row("ab "));
        assert!(out.is_empty());
        diff_row(&mut out, 2, &row("ab "), &row("ab cd "));
        assert_eq!(out, b"\x1b[3;4Hcd ");
        out.clear();
        diff_row(&mut out, 2, &row("ab cd "), &row("xy "));
        assert_eq!(out, b"\x1b[3;1Hxy \x1b[K");
    }
}
