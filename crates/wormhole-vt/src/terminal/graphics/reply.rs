//! What the terminal answers a graphics command, and keeping the answers in order.
//!
//! Spec (kitty, "Terminal Responses"): a command that carries an image id or number is answered
//! `ESC _ G i=<id>[,I=<number>][,p=<placement>] ; <message> ESC \`, the message `OK` or an error
//! code and text (`ENOENT:…`), unless the program turned it off with `q`. A command with neither
//! is never answered.
//!
//! Most answers are known when the command ends. One is not: a PNG or a compressed picture is
//! only known to be good once the host has decoded it ([`Terminal::image_decoded`]). Programs
//! read their answers in the order they asked, so while one waits every later reply of any kind
//! (a cursor report, say) is held behind it in a [`Gate`] and let through in order when it is
//! settled.
//!
//! [`Terminal::image_decoded`]: super::super::Terminal::image_decoded

use std::collections::VecDeque;
use std::io::Write as _;

use crate::types::ImageHandle;

use super::command::{Command, Quiet};

/// Why a command was refused, with the code the protocol names for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    /// A key or value the protocol does not allow (`EINVAL`).
    Invalid(&'static str),
    /// More than this terminal keeps (`EFBIG`).
    TooBig,
    /// No such image (`ENOENT`).
    NotFound,
    /// The data is not the picture it says it is (`ENODATA`).
    BadData(&'static str),
    /// A file, shared memory or a feature this terminal does not do (`EBADF`: as every file
    /// failure must answer alike, so that a program learns nothing about files it cannot read).
    Unsupported(&'static str),
}

impl Error {
    fn code(self) -> &'static str {
        match self {
            Error::Invalid(_) => "EINVAL",
            Error::TooBig => "EFBIG",
            Error::NotFound => "ENOENT",
            Error::BadData(_) => "ENODATA",
            Error::Unsupported(_) => "EBADF",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Error::Invalid(what) | Error::BadData(what) | Error::Unsupported(what) => what,
            Error::TooBig => "the image is bigger than this terminal keeps",
            Error::NotFound => "no image with that id",
        }
    }
}

/// Whom a reply is for: the ids of the command, and which replies it wants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Target {
    pub id: u32,
    pub number: u32,
    pub placement: u32,
    pub quiet: Quiet,
}

impl Target {
    pub fn of(command: &Command) -> Target {
        Target { id: command.id, number: command.number, placement: command.placement, quiet: command.quiet }
    }

    /// The same, for a picture the terminal gave an id (a numbered one).
    pub fn with_id(self, id: u32) -> Target {
        Target { id, ..self }
    }

    /// The bytes of the reply, `None` when there is none to send.
    pub fn render(self, result: Result<(), Error>) -> Option<Vec<u8>> {
        if (self.id == 0 && self.number == 0)
            || (result.is_ok() && self.quiet != Quiet::Normal)
            || (result.is_err() && self.quiet == Quiet::Silent)
        {
            return None;
        }
        let mut out = Vec::with_capacity(64);
        let _ = out.write_all(b"\x1b_G");
        let mut keys = Vec::with_capacity(3);
        if self.id != 0 {
            keys.push(format!("i={}", self.id));
        }
        if self.number != 0 {
            keys.push(format!("I={}", self.number));
        }
        if self.placement != 0 {
            keys.push(format!("p={}", self.placement));
        }
        let _ = out.write_all(keys.join(",").as_bytes());
        let _ = match result {
            Ok(()) => out.write_all(b";OK"),
            Err(err) => write!(out, ";{}:{}", err.code(), err.message()),
        };
        let _ = out.write_all(b"\x1b\\");
        Some(out)
    }
}

/// One piece of the replies in line: bytes ready to go, or the place of an answer that waits.
#[derive(Debug)]
enum Part {
    Ready(Vec<u8>),
    Waiting(ImageHandle, Target),
}

/// Holds replies behind an answer that is not known yet; see the module docs.
#[derive(Debug, Default)]
pub(super) struct Gate {
    parts: VecDeque<Part>,
    held: usize,
    /// The reply being held went past the cap: its other pieces go too.
    dropped: bool,
}

/// The most bytes held back before the oldest waiting answer is given up on, so a host that
/// never reports a decode cannot make the terminal hold replies without end.
const HELD_MAX: usize = 1 << 20;
/// The most answers waiting at once.
const WAITING_MAX: usize = 64;

impl Gate {
    /// Whether replies are being held (something waits).
    #[inline]
    pub fn is_closed(&self) -> bool {
        !self.parts.is_empty()
    }

    /// Hold the bytes of an ordinary reply behind the answers that wait.
    pub fn hold(&mut self, args: std::fmt::Arguments<'_>) {
        self.hold_with(|bytes| {
            let _ = bytes.write_fmt(args);
        });
    }

    /// Hold ready-made bytes, as [`hold`](Self::hold) does.
    pub fn hold_bytes(&mut self, data: &[u8]) {
        self.hold_with(|bytes| bytes.extend_from_slice(data));
    }

    fn hold_with(&mut self, put: impl FnOnce(&mut Vec<u8>)) {
        if !matches!(self.parts.back(), Some(Part::Ready(_))) {
            self.parts.push_back(Part::Ready(Vec::new()));
        }
        if let Some(Part::Ready(bytes)) = self.parts.back_mut() {
            // As `Terminal::reply`: a reply past the cap is dropped whole, decided at its ESC.
            let before = bytes.len();
            put(bytes);
            if bytes.get(before) == Some(&0x1B) {
                self.dropped = self.held >= HELD_MAX;
            }
            // Pieces that never start a reply still stop somewhere.
            if self.dropped || self.held >= 2 * HELD_MAX {
                bytes.truncate(before);
            } else {
                self.held += bytes.len() - before;
            }
        }
    }

    /// An answer for `image` that waits for the host. Past the caps the oldest waiting answer
    /// is let go as an error instead (returned, for the caller to send).
    pub fn wait(&mut self, image: ImageHandle, target: Target) -> Vec<Vec<u8>> {
        self.parts.push_back(Part::Waiting(image, target));
        let waiting = self.parts.iter().filter(|part| matches!(part, Part::Waiting(..))).count();
        let oldest = self.parts.iter().find_map(|part| match part {
            Part::Waiting(image, _) => Some(*image),
            Part::Ready(_) => None,
        });
        if let Some(oldest) = oldest.filter(|_| waiting > WAITING_MAX || self.held >= HELD_MAX) {
            return self.settle(oldest, Err(Error::BadData("the picture was not decoded in time")));
        }
        Vec::new()
    }

    /// The answer for `image` is known. Returns the bytes now free to go, in order.
    pub fn settle(&mut self, image: ImageHandle, result: Result<(), Error>) -> Vec<Vec<u8>> {
        if let Some(at) =
            self.parts.iter().position(|part| matches!(part, Part::Waiting(h, _) if *h == image))
        {
            let reply = match self.parts.remove(at) {
                Some(Part::Waiting(_, target)) => target.render(result).unwrap_or_default(),
                _ => Vec::new(),
            };
            self.parts.insert(at, Part::Ready(reply));
        }
        let mut out = Vec::new();
        while let Some(Part::Ready(_)) = self.parts.front() {
            if let Some(Part::Ready(bytes)) = self.parts.pop_front() {
                self.held = self.held.saturating_sub(bytes.len());
                out.push(bytes);
            }
        }
        out
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.held = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Gate, Target};
    use crate::terminal::graphics::command::Quiet;
    use crate::types::ImageHandle;

    fn target(id: u32) -> Target {
        Target { id, ..Target::default() }
    }

    fn text(bytes: Option<Vec<u8>>) -> String {
        String::from_utf8(bytes.unwrap_or_default()).unwrap_or_default()
    }

    #[test]
    fn answers_name_the_ids_and_the_code() {
        assert_eq!(text(target(3).render(Ok(()))), "\x1b_Gi=3;OK\x1b\\");
        let numbered = Target { id: 4, number: 5, placement: 6, quiet: Quiet::Normal };
        assert_eq!(text(numbered.render(Ok(()))), "\x1b_Gi=4,I=5,p=6;OK\x1b\\");
        assert_eq!(
            text(target(3).render(Err(Error::NotFound))),
            "\x1b_Gi=3;ENOENT:no image with that id\x1b\\"
        );
        assert_eq!(
            text(Target { number: 5, ..Target::default() }.render(Err(Error::TooBig))).as_str().get(..9),
            Some("\x1b_GI=5;EF")
        );
        assert_eq!(Target::default().render(Ok(())), None, "no id, no answer");
        assert!(text(target(1).render(Err(Error::Invalid("x")))).contains("EINVAL:x"));
        assert!(text(target(1).render(Err(Error::BadData("x")))).contains("ENODATA:x"));
        assert!(text(target(1).render(Err(Error::Unsupported("x")))).contains("EBADF:x"));
    }

    #[test]
    fn quiet_keeps_answers_back() {
        let no_ok = Target { id: 3, quiet: Quiet::NoOk, ..Target::default() };
        assert_eq!(no_ok.render(Ok(())), None);
        assert!(no_ok.render(Err(Error::TooBig)).is_some());
        let silent = Target { id: 3, quiet: Quiet::Silent, ..Target::default() };
        assert_eq!(silent.render(Err(Error::TooBig)), None);
    }

    #[test]
    fn later_replies_wait_behind_an_answer_that_waits() {
        let mut gate = Gate::default();
        assert!(!gate.is_closed());
        assert!(gate.wait(ImageHandle(1), target(1)).is_empty());
        assert!(gate.is_closed());
        gate.hold(format_args!("\x1b[0n"));
        gate.hold(format_args!("\x1b[?62c"));
        assert!(gate.wait(ImageHandle(2), target(2)).is_empty());
        gate.hold(format_args!("later"));
        // The second settles first: nothing is free until the first does.
        assert!(gate.settle(ImageHandle(2), Ok(())).is_empty());
        let out: Vec<String> = gate
            .settle(ImageHandle(1), Err(Error::BadData("bad")))
            .into_iter()
            .map(|b| text(Some(b)))
            .collect();
        assert_eq!(out.concat(), "\x1b_Gi=1;ENODATA:bad\x1b\\\x1b[0n\x1b[?62c\x1b_Gi=2;OK\x1b\\later");
        assert!(!gate.is_closed());
    }

    #[test]
    fn a_host_that_never_answers_cannot_hold_replies_for_ever() {
        let mut gate = Gate::default();
        let mut freed = 0;
        for n in 0..100u64 {
            freed += gate.wait(ImageHandle(n), target(n as u32 + 1)).len();
        }
        assert!(freed > 0, "the oldest were given up on");
        let mut gate = Gate::default();
        gate.wait(ImageHandle(1), target(1));
        for _ in 0..(super::HELD_MAX / 4 + 10) {
            gate.hold(format_args!("\x1b[0n"));
        }
        assert!(gate.held <= super::HELD_MAX + 4);
        for _ in 0..super::HELD_MAX {
            gate.hold(format_args!("abcd"));
        }
        assert!(gate.held <= 2 * super::HELD_MAX + 4);
    }
}
