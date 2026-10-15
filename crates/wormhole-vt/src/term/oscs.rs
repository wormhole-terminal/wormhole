//! What each operating system command does.
//!
//! Spec: xterm's ctlseqs ("Operating System Commands": 0, 1, 2 titles, 52 clipboard; the colors are in `osc_colors`), the OSC 8 hyperlink specification, FinalTerm's OSC 133
//! (shell marks), iTerm2's OSC 9 and 1337, ConEmu's OSC 9;4 progress, rxvt's OSC 777 `notify`,
//! kitty's OSC 99. The ones whose protocol is big and not the terminal's business are handed to
//! the host raw: 99, 7501 and what is not an inline image in 1337.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::grid::Content;
use crate::types::Hyperlink;

use super::base64;
use super::event::{Event, Progress, ProgressState, PromptKind, ShellMark, Terminator};
use super::osc::split_number;
use super::{CWD_MAX, CWD_REPORTS_MAX, RECENT_LINKS, Terminal};

/// The terminator a reply to an OSC ends with: the one the request used.
fn terminator(end: Terminator) -> &'static str {
    match end {
        Terminator::Bel => "\x07",
        Terminator::St => "\x1b\\",
    }
}

/// A short text built without allocating: `write!` into it, then read it back.
struct StackText {
    bytes: [u8; 40],
    len: usize,
}

impl StackText {
    fn new() -> StackText {
        StackText { bytes: [0; 40], len: 0 }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for StackText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len + text.len();
        self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Cut `text` to at most `max` bytes, at a character boundary.
fn cut(text: &mut String, max: usize) {
    if text.len() > max {
        let mut at = max;
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        text.truncate(at);
    }
}

pub(super) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Replace `text`'s contents with `bytes` (invalid UTF-8 as U+FFFD), cut to `max` bytes at a
/// character boundary. Reuses the string's buffer: a title or folder that changes with every
/// prompt costs no allocation.
pub(super) fn fill(text: &mut String, bytes: &[u8], max: usize) {
    text.clear();
    match std::str::from_utf8(bytes) {
        Ok(valid) => text.push_str(valid),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    cut(text, max);
}

impl Terminal {
    /// The OSC string has ended.
    pub(super) fn finish_osc(&mut self, end: Terminator) {
        let Some(mut buf) = self.osc.take() else { return };
        if let Some((number, rest)) = split_number(&buf) {
            let skip = buf.len() - rest.len();
            self.run_osc(number, &mut buf, skip, end);
        }
        self.osc.recycle(buf);
    }

    fn run_osc(&mut self, number: u32, buf: &mut Vec<u8>, skip: usize, end: Terminator) {
        let rest = &buf[skip.min(buf.len())..];
        match number {
            0 => self.set_titles(true, true, rest),
            1 => self.set_titles(false, true, rest),
            2 => self.set_titles(true, false, rest),
            4 | 5 => self.osc_palette(number, rest, end),
            7 => self.set_cwd(rest),
            8 => self.osc_hyperlink(rest),
            9 => self.osc_9(rest),
            10..=19 => self.osc_dynamic_colors(number, rest, end),
            52 => self.osc_clipboard(rest, end),
            99 => {
                let (metadata, payload) = match rest.iter().position(|&b| b == b';') {
                    Some(at) => (&rest[..at], &rest[at + 1..]),
                    None => (rest, &rest[rest.len()..]),
                };
                self.push_event(Event::KittyNotification {
                    metadata: lossy(metadata),
                    payload: lossy(payload),
                });
            }
            104 => self.osc_reset_palette(rest),
            105 => self.osc_reset_special(rest),
            110..=119 => self.osc_reset_dynamic(number),
            133 => self.osc_shell_mark(rest),
            777 => self.osc_777(rest),
            1337 => {
                // Inline images are the engine's; the rest of iTerm2's protocol is the host's.
                if !self.iterm_image(rest) {
                    buf.drain(..skip);
                    self.push_event(Event::Iterm(std::mem::take(buf)));
                }
            }
            7501 => {
                buf.drain(..skip);
                self.push_event(Event::ProgramStatus { payload: std::mem::take(buf), terminator: end });
            }
            _ => {}
        }
    }

    /// OSC 7.
    pub(super) fn set_cwd(&mut self, text: &[u8]) {
        fill(&mut self.cwd, text, CWD_MAX);
        // The report is kept in a buffer that is reused, so a prompt that says it every time
        // allocates nothing once the host has taken the first.
        if self.cwd_reported < self.cwd_reports.len() {
            self.cwd_reports[self.cwd_reported].clone_from(&self.cwd);
        } else {
            self.cwd_reports.push(self.cwd.clone());
        }
        self.cwd_reported = (self.cwd_reported + 1).min(CWD_REPORTS_MAX);
        self.push_event(Event::Cwd);
    }

    /// The folders reported (OSC 7) since [`clear_cwd_reports`](Self::clear_cwd_reports), oldest
    /// first, as written (`file://host/path`, percent-encoded; ConEmu's a bare path). At most the
    /// last [`CWD_REPORTS_MAX`] are kept.
    pub fn cwd_reports(&self) -> &[String] {
        &self.cwd_reports[..self.cwd_reported]
    }

    /// Forget the reports, after reading them; their buffers are kept.
    pub fn clear_cwd_reports(&mut self) {
        self.cwd_reported = 0;
    }

    // ---- hyperlinks ----

    /// OSC 8 `params ; uri`: the cells printed from here on are a link, until an empty `uri`.
    fn osc_hyperlink(&mut self, rest: &[u8]) {
        let Some(at) = rest.iter().position(|&b| b == b';') else { return };
        let (params, uri) = (&rest[..at], &rest[at + 1..]);
        if uri.is_empty() {
            self.pen.link = None;
            return;
        }
        // The target is text a user may be shown and click: no control characters.
        let Ok(uri) = std::str::from_utf8(uri) else { return };
        if uri.chars().any(char::is_control) {
            return;
        }
        let id = params
            .split(|&b| b == b':')
            .find_map(|p| p.strip_prefix(b"id="))
            .and_then(|id| std::str::from_utf8(id).ok().filter(|id| !id.is_empty()));
        if let (Some(id), Some(open)) = (id, &self.pen.link)
            && &*open.id == id
            && &*open.uri == uri
        {
            return;
        }
        self.pen.link = Some(self.intern_link(id, uri));
    }

    /// The link `id` (or none) and `uri` name. The text of the last few is kept, so that a
    /// program that prints the same file name or address again and again (a build log, a file
    /// listing, an agent naming the files it touched) shares one allocation of it; a link with no
    /// id of its own gets a fresh id, since nothing ties two of them together.
    fn intern_link(&mut self, id: Option<&str>, uri: &str) -> Hyperlink {
        let hit = self
            .recent_links
            .iter()
            .position(|link| &*link.uri == uri && id.is_none_or(|id| &*link.id == id));
        let shared_uri = hit.map(|at| {
            // Most recent first.
            let link = self.recent_links.remove(at);
            self.recent_links.insert(0, link.clone());
            link
        });
        if let (Some(link), Some(_)) = (&shared_uri, id) {
            return link.clone();
        }
        let uri: Arc<str> = shared_uri.map_or_else(|| Arc::from(uri), |link| link.uri);
        let id: Arc<str> = match id {
            Some(id) => Arc::from(id),
            None => {
                self.link_seq += 1;
                let mut name = StackText::new();
                let _ = write!(name, "wormhole-link-{}", self.link_seq);
                Arc::from(name.as_str())
            }
        };
        let link = Hyperlink { id, uri };
        self.recent_links.truncate(RECENT_LINKS - 1);
        self.recent_links.insert(0, link.clone());
        link
    }

    // ---- clipboard ----

    /// OSC 52 `selection ; data`: `data` is base64 text to copy, or `?` to ask for the clipboard.
    fn osc_clipboard(&mut self, rest: &[u8], end: Terminator) {
        let (selection, data) = match rest.iter().position(|&b| b == b';') {
            Some(at) => (&rest[..at], &rest[at + 1..]),
            None => return,
        };
        // xterm: an empty selection is the primary one and cut buffer 0 (`s0`), and the reply says so.
        let selection = if selection.is_empty() { "s0".to_string() } else { lossy(selection) };
        if data == b"?" {
            let replies_before = self.replies.len();
            self.push_event(Event::ClipboardLoad { selection, terminator: end, replies_before });
        } else if let Some(bytes) = base64::decode(data) {
            let text = String::from_utf8(bytes).unwrap_or_else(|error| lossy(error.as_bytes()));
            self.push_event(Event::ClipboardStore { selection, text });
        }
    }

    /// The reply to an [`Event::ClipboardLoad`]: the clipboard's `text`, for `selection`, ended
    /// as the request was.
    pub fn clipboard_reply(&mut self, selection: &str, text: &str, end: Terminator) {
        let data = base64::encode(text.as_bytes());
        self.reply(format_args!("\x1b]52;{selection};{data}{}", terminator(end)));
    }

    // ---- notifications and progress ----

    /// OSC 9: iTerm2's notification, ConEmu's numbered commands (of which `4` is progress and `9`
    /// the working folder).
    fn osc_9(&mut self, rest: &[u8]) {
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

    /// OSC 777 `notify ; title ; body`.
    fn osc_777(&mut self, rest: &[u8]) {
        let mut parts = rest.splitn(3, |&b| b == b';');
        if parts.next() != Some(b"notify") {
            return;
        }
        let title = lossy(parts.next().unwrap_or(&[]));
        let body = lossy(parts.next().unwrap_or(&[]));
        self.push_event(Event::Notification { title, body });
    }

    // ---- shell marks ----

    /// OSC 133: `A` prompt, `B` command line, `C` output, `D` finished. The pen carries the mark
    /// so every cell printed afterwards belongs to the block; the rows learn their block from it.
    fn osc_shell_mark(&mut self, rest: &[u8]) {
        let mut parts = rest.split(|&b| b == b';');
        let mark = match parts.next() {
            Some(b"A") => {
                let kind = match parts.find_map(|p| p.strip_prefix(b"k=")) {
                    Some(b"s") => PromptKind::Secondary,
                    Some(b"r") => PromptKind::Right,
                    Some(b"c") => PromptKind::Continuation,
                    _ => PromptKind::Initial,
                };
                if kind == PromptKind::Initial {
                    self.block = self.block.wrapping_add(1).max(1);
                    self.pen.block = self.block;
                    if self.block_row_is_blank() {
                        self.grid.set_row_block(self.cursor.y, self.block);
                    }
                }
                if self.pen.block != 0 {
                    self.pen.content = Content::Prompt;
                }
                ShellMark::PromptStart(kind)
            }
            Some(b"B") => {
                self.set_content(Content::Command);
                ShellMark::CommandStart
            }
            Some(b"C") => {
                self.set_content(Content::Output);
                ShellMark::OutputStart
            }
            Some(b"D") => {
                self.pen.content = Content::None;
                let code =
                    parts.next().and_then(|c| std::str::from_utf8(c).ok()).and_then(|c| c.parse().ok());
                ShellMark::Finished(code)
            }
            _ => return,
        };
        self.push_event(Event::Mark(mark));
    }

    /// Whether the cursor's row has nothing on it yet.
    fn block_row_is_blank(&self) -> bool {
        self.grid.content_len(crate::types::Line(self.cursor.y as i32)) == 0
    }

    fn set_content(&mut self, content: Content) {
        if self.pen.block != 0 && !self.on_alt_screen() {
            self.pen.content = content;
        }
    }
}
