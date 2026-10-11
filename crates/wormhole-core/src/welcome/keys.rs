//! What the person types, read from the terminal's bytes: the arrows, Return, Esc, Tab, letters,
//! and SGR mouse clicks (`ESC [ < b ; x ; y M`). The program asks for nothing fancier, so it works the
//! same in any terminal that speaks xterm.
//!
//! A lone Esc is the one ambiguous byte: it is a key by itself and the start of every arrow. The
//! parser keeps it until more bytes arrive; the caller's tick calls [`Parser::flush`] once none
//! have for [`ESC_WAIT`], and the Esc is a key.

use std::time::Duration;

/// How long a lone Esc waits for the rest of an escape sequence.
pub const ESC_WAIT: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    /// Ctrl-C, which raw mode hands over as a byte instead of a signal.
    Interrupt,
    Char(char),
    /// A left click, 0-based from the screen's top left.
    Click {
        row: usize,
        col: usize,
    },
}

/// Turns bytes into keys, holding what is not whole yet.
#[derive(Debug, Default)]
pub struct Parser {
    pending: Vec<u8>,
}

impl Parser {
    /// Feed `bytes`, get the keys they complete.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Key> {
        self.pending.extend_from_slice(bytes);
        let mut keys = Vec::new();
        loop {
            match self.next_key() {
                Step::Key(key, used) => {
                    keys.push(key);
                    self.pending.drain(..used);
                }
                Step::Skip(used) => {
                    self.pending.drain(..used);
                }
                Step::More => break,
            }
        }
        // A runaway sequence (a terminal that never finishes one) is dropped, not kept for ever.
        if self.pending.len() > 64 {
            self.pending.clear();
        }
        keys
    }

    /// No more bytes came: a lone Esc is the Esc key.
    pub fn flush(&mut self) -> Vec<Key> {
        if self.pending == [0x1b] {
            self.pending.clear();
            return vec![Key::Esc];
        }
        Vec::new()
    }

    /// Whether bytes of an unfinished sequence are being held.
    pub fn holding(&self) -> bool {
        !self.pending.is_empty()
    }

    fn next_key(&self) -> Step {
        let bytes = &self.pending[..];
        let Some(&first) = bytes.first() else { return Step::More };
        match first {
            0x1b => escape(bytes),
            b'\r' | b'\n' => Step::Key(Key::Enter, 1),
            b'\t' => Step::Key(Key::Tab, 1),
            0x7f | 0x08 => Step::Key(Key::Backspace, 1),
            0x03 | 0x04 => Step::Key(Key::Interrupt, 1),
            0x00..=0x1f => Step::Skip(1),
            _ => text(bytes),
        }
    }
}

enum Step {
    Key(Key, usize),
    Skip(usize),
    /// An incomplete sequence: wait for more.
    More,
}

/// A printable character, possibly several bytes of UTF-8.
fn text(bytes: &[u8]) -> Step {
    let need = match bytes[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        // A stray continuation byte.
        _ => return Step::Skip(1),
    };
    if bytes.len() < need {
        return Step::More;
    }
    match std::str::from_utf8(&bytes[..need]).ok().and_then(|s| s.chars().next()) {
        Some(c) => Step::Key(Key::Char(c), need),
        None => Step::Skip(1),
    }
}

/// What starts with Esc: an arrow (`ESC [ A`, `ESC O A`), Shift-Tab, a click, or another sequence to
/// skip whole.
fn escape(bytes: &[u8]) -> Step {
    let Some(&second) = bytes.get(1) else { return Step::More };
    match second {
        b'[' => csi(bytes),
        b'O' => match bytes.get(2) {
            None => Step::More,
            Some(b'A') => Step::Key(Key::Up, 3),
            Some(b'B') => Step::Key(Key::Down, 3),
            Some(b'C') => Step::Key(Key::Right, 3),
            Some(b'D') => Step::Key(Key::Left, 3),
            Some(_) => Step::Skip(3),
        },
        // Esc then something else: Alt+key, or Esc typed before a key. The Esc counts; the key is
        // read next.
        _ => Step::Key(Key::Esc, 1),
    }
}

fn csi(bytes: &[u8]) -> Step {
    // Parameters and intermediates, then a final byte in 0x40..=0x7e.
    let Some(end) = bytes[2..].iter().position(|b| (0x40..=0x7e).contains(b)) else { return Step::More };
    let end = end + 2;
    let params = &bytes[2..end];
    let used = end + 1;
    // The arrows have no parameters: with one (`1;5A`) a modifier is held, which is not ours.
    let key = match bytes[end] {
        b'A' if params.is_empty() => Some(Key::Up),
        b'B' if params.is_empty() => Some(Key::Down),
        b'C' if params.is_empty() => Some(Key::Right),
        b'D' if params.is_empty() => Some(Key::Left),
        b'Z' if params.is_empty() => Some(Key::BackTab),
        b'M' | b'm' if params.first() == Some(&b'<') => click(&params[1..], bytes[end] == b'M'),
        _ => None,
    };
    match key {
        Some(key) => Step::Key(key, used),
        None => Step::Skip(used),
    }
}

/// `b;x;y` of an SGR mouse report: a press of the left button (`M`, button 0, no motion or
/// modifier). Releases, other buttons, wheels and drags are not clicks.
fn click(params: &[u8], press: bool) -> Option<Key> {
    let text = std::str::from_utf8(params).ok()?;
    let mut numbers = text.split(';').map(|n| n.parse::<usize>().ok());
    let (button, col, row) = (numbers.next()??, numbers.next()??, numbers.next()??);
    (press && button == 0 && col >= 1 && row >= 1).then(|| Key::Click { row: row - 1, col: col - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<Key> {
        Parser::default().feed(bytes)
    }

    #[test]
    fn the_arrows_return_and_letters_are_read() {
        assert_eq!(keys(b"\x1b[A\x1b[B\x1b[C\x1b[D"), [Key::Up, Key::Down, Key::Right, Key::Left]);
        assert_eq!(keys(b"\x1bOA\x1bOD"), [Key::Up, Key::Left], "application cursor keys too");
        assert_eq!(keys(b"\r"), [Key::Enter]);
        assert_eq!(keys(b"\n"), [Key::Enter]);
        assert_eq!(keys(b"\t\x1b[Z"), [Key::Tab, Key::BackTab]);
        assert_eq!(keys(b"pl+"), [Key::Char('p'), Key::Char('l'), Key::Char('+')]);
        assert_eq!(keys("é日".as_bytes()), [Key::Char('é'), Key::Char('日')]);
        assert_eq!(keys(b"\x7f"), [Key::Backspace]);
        assert_eq!(keys(b"\x03"), [Key::Interrupt]);
    }

    #[test]
    fn a_lone_escape_waits_to_see_whether_it_is_an_arrow() {
        let mut parser = Parser::default();
        assert!(parser.feed(b"\x1b").is_empty());
        assert!(parser.holding());
        assert_eq!(parser.feed(b"[A"), [Key::Up], "it was the start of an arrow");
        assert!(!parser.holding());
        assert!(parser.feed(b"\x1b").is_empty());
        assert_eq!(parser.flush(), [Key::Esc], "no more came: it is the key");
        assert!(parser.flush().is_empty());
        // Sequences split across reads are put together.
        assert!(parser.feed(b"\x1b[").is_empty());
        assert_eq!(parser.feed(b"B"), [Key::Down]);
    }

    #[test]
    fn a_click_is_a_left_press_and_nothing_else_is() {
        assert_eq!(keys(b"\x1b[<0;12;5M"), [Key::Click { row: 4, col: 11 }]);
        assert!(keys(b"\x1b[<0;12;5m").is_empty(), "the release");
        assert!(keys(b"\x1b[<2;12;5M").is_empty(), "the right button");
        assert!(keys(b"\x1b[<64;12;5M").is_empty(), "the wheel");
        assert!(keys(b"\x1b[<32;12;5M").is_empty(), "a drag");
        assert!(keys(b"\x1b[<0;0;0M").is_empty(), "no such cell");
        // A garbled report ends at its first final byte (`x`); what is left is typed text, not a click.
        assert!(keys(b"\x1b[<0;x;5M").iter().all(|key| !matches!(key, Key::Click { .. })));
    }

    #[test]
    fn what_is_not_understood_is_skipped_whole_and_the_next_key_still_reads() {
        assert_eq!(keys(b"\x1b[5~a"), [Key::Char('a')], "Page Up is skipped as a unit");
        assert_eq!(keys(b"\x1b[1;5Ax"), [Key::Char('x')], "Ctrl+Up is not an Up");
        assert_eq!(keys(b"\x00\x01q"), [Key::Char('q')]);
        assert_eq!(keys(b"\xffz"), [Key::Char('z')], "a stray byte");
        assert_eq!(keys(b"\x1bx"), [Key::Esc, Key::Char('x')], "Esc then a key");
    }

    #[test]
    fn a_sequence_that_never_ends_is_dropped() {
        let mut parser = Parser::default();
        let junk = vec![b'0'; 100];
        assert!(parser.feed(b"\x1b[").is_empty());
        assert!(parser.feed(&junk).is_empty());
        assert!(!parser.holding(), "not kept for ever");
        assert_eq!(parser.feed(b"a"), [Key::Char('a')]);
    }
}
