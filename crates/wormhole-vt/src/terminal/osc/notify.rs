//! The notifications and the progress a program can send: OSC 9 (iTerm2's notification, and
//! ConEmu's numbered commands), OSC 9;4 (progress), OSC 777 `notify` (rxvt's) and OSC 99 (kitty's,
//! handed to the host raw).

use crate::terminal::Terminal;
use crate::terminal::event::{Event, Progress, ProgressState};
use crate::terminal::osc::dispatch::lossy;

impl Terminal {
    /// OSC 9: iTerm2's notification, ConEmu's numbered commands (of which `4` is progress and `9`
    /// the working folder).
    pub(in crate::terminal) fn osc_9(&mut self, rest: &[u8]) {
        let (head, tail) = match rest.iter().position(|&b| b == b';') {
            Some(at) => (&rest[..at], Some(&rest[at + 1..])),
            None => (rest, None),
        };
        let conemu = (1..=2).contains(&head.len()) && head.iter().all(u8::is_ascii_digit);
        if conemu {
            match (head, tail) {
                (b"4", tail) => self.osc_progress(tail.unwrap_or(&[])),
                (b"9", Some(path)) => {
                    let path = path.strip_prefix(b"\"").and_then(|p| p.strip_suffix(b"\"")).unwrap_or(path);
                    self.set_cwd(path);
                }
                _ => {}
            }
        } else if !rest.is_empty() {
            self.push_event(Event::Notification { title: String::new(), body: lossy(rest) });
        }
    }

    /// OSC 9;4 `state ; percent`.
    fn osc_progress(&mut self, rest: &[u8]) {
        let mut parts = rest.split(|&b| b == b';');
        let state = match parts.next() {
            Some(b"0") => {
                self.push_event(Event::Progress(None));
                return;
            }
            Some(b"1") => ProgressState::Normal,
            Some(b"2") => ProgressState::Error,
            Some(b"3") => ProgressState::Indeterminate,
            Some(b"4") => ProgressState::Paused,
            _ => return,
        };
        let percent = parts
            .next()
            .and_then(|p| std::str::from_utf8(p).ok())
            .and_then(|p| p.trim().parse::<u32>().ok())
            .map_or(0, |p| p.min(100) as u8);
        self.push_event(Event::Progress(Some(Progress { state, percent })));
    }

    /// OSC 99: kitty's notification protocol is the host's; it gets the metadata and the payload.
    pub(in crate::terminal) fn osc_99(&mut self, rest: &[u8]) {
        let (metadata, payload) = match rest.iter().position(|&b| b == b';') {
            Some(at) => (&rest[..at], &rest[at + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        self.push_event(Event::KittyNotification { metadata: lossy(metadata), payload: lossy(payload) });
    }

    /// OSC 777 `notify ; title ; body`.
    pub(in crate::terminal) fn osc_777(&mut self, rest: &[u8]) {
        let mut parts = rest.splitn(3, |&b| b == b';');
        if parts.next() != Some(b"notify") {
            return;
        }
        let title = lossy(parts.next().unwrap_or(&[]));
        let body = lossy(parts.next().unwrap_or(&[]));
        self.push_event(Event::Notification { title, body });
    }
}
