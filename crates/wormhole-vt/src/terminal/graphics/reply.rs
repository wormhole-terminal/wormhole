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
//! (a cursor report, say) is held behind it and let through in order when it is settled; that
//! queue is `reports::queue::ReplyQueue`, which also renders the answer from its [`Target`].
//!
//! [`Terminal::image_decoded`]: super::super::Terminal::image_decoded

use std::io::Write as _;

use super::command::{Command, Quiet};

/// Why a command was refused, with the code the protocol names for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Error {
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
pub(in crate::terminal) struct Target {
    pub(super) id: u32,
    pub(super) number: u32,
    pub(super) placement: u32,
    pub(super) quiet: Quiet,
}

impl Target {
    pub(super) fn of(command: &Command) -> Target {
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

#[cfg(test)]
mod tests {
    use super::{Error, Target};
    use crate::terminal::graphics::command::Quiet;

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
}
