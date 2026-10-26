//! A read the socket thread carries out itself: `read` and `blocks` copy the terminal's screen under
//! its lock and turn it into the reply, which is work for the thread that holds the connection, not
//! for the UI thread that decided the caller may have it.
//!
//! The app resolves the target and the policy on its thread in a moment and hands over a
//! [`ReadJob`] holding the terminal's [`ScreenReader`]; the socket thread runs it with the
//! [`ReadScratch`] it keeps between jobs, so a read allocates only what it returns.

use wormhole_term::{BlocksSpec, CaptureError, CaptureSpec, ReadScratch, ScreenReader};

use super::control::printable::json_line;
use super::control::read::{Blocks, REPLY_MAX_BYTES, Read};
use super::control::{Error, ErrorCode, Reason, Reply};

/// What the reply has beside the result's own bytes, with room to spare.
const ENVELOPE: usize = 1024;

/// A read to carry out.
pub struct ReadJob {
    reader: ScreenReader,
    terminal: i64,
    what: What,
}

impl std::fmt::Debug for ReadJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadJob").field("terminal", &self.terminal).finish_non_exhaustive()
    }
}

enum What {
    Lines { title: Option<String>, spec: CaptureSpec },
    Blocks(BlocksSpec),
}

impl ReadJob {
    /// The lines `spec` names of `terminal`, with its `title` (terminal text the app knows).
    pub fn lines(reader: ScreenReader, terminal: i64, title: Option<String>, spec: CaptureSpec) -> Self {
        Self { reader, terminal, what: What::Lines { title, spec } }
    }

    /// The newest blocks of `terminal`.
    pub fn blocks(reader: ScreenReader, terminal: i64, spec: BlocksSpec) -> Self {
        Self { reader, terminal, what: What::Blocks(spec) }
    }

    /// Do the read: the reply, which is within [`REPLY_MAX_BYTES`] (the oldest lines, or the oldest
    /// blocks' output, are dropped, and it says so).
    pub fn run(self, scratch: &mut ReadScratch) -> Reply {
        match self.what {
            What::Lines { title, spec } => match self.reader.capture(&spec, scratch) {
                Ok(capture) => fit_lines(Read::of(self.terminal, title, capture)),
                Err(CaptureError::NoIntegration) => Error::new(
                    ErrorCode::Unsupported,
                    "this shell has no integration, so it has no command blocks to read",
                )
                .because(Reason::NoIntegration)
                .into(),
                Err(missing) => Reply::error(ErrorCode::NotFound, missing.to_string()),
            },
            What::Blocks(spec) => fit_blocks(Blocks::of(self.terminal, &self.reader.blocks(&spec, scratch))),
        }
    }
}

/// The reply for `read`, with lines dropped from the top while it is over the cap.
fn fit_lines(mut read: Read) -> Reply {
    let limit = REPLY_MAX_BYTES - ENVELOPE;
    if read.size_estimate() > limit {
        while json_line(&read).len() > limit && !read.lines.is_empty() {
            read.drop_oldest();
        }
    }
    Reply::ok(read)
}

/// The reply for `blocks`, with the oldest outputs dropped while it is over the cap.
fn fit_blocks(mut blocks: Blocks) -> Reply {
    let limit = REPLY_MAX_BYTES - ENVELOPE;
    if blocks.size_estimate() > limit {
        while json_line(&blocks).len() > limit && blocks.drop_oldest_output() {}
    }
    Reply::ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::{REPLY_MAX_BYTES, fit_blocks, fit_lines};
    use crate::ipc::control::printable::json_line;
    use crate::ipc::control::read::{BlockInfo, Blocks, CursorInfo, ModesInfo, Read, Size};

    fn read(lines: Vec<String>) -> Read {
        Read {
            terminal: 1,
            title: None,
            screen: "main".into(),
            size: Size { rows: 10, cols: 80 },
            history: 0,
            cursor: CursorInfo { row: 0, col: 0, visible: true, shape: None },
            modes: ModesInfo {
                app_cursor: false,
                app_keypad: false,
                bracketed_paste: false,
                locator: false,
                mouse: "off".into(),
                mouse_encoding: "x10".into(),
                kitty_keyboard: 0,
            },
            first_line: 0,
            lines,
            styles: None,
            truncated: false,
            revision: 0,
            block: None,
        }
    }

    #[test]
    fn a_read_over_the_reply_cap_drops_its_oldest_lines_and_says_so() {
        // Invisible characters are written as six-byte escapes, so the text is smaller than its reply.
        let lines: Vec<String> = (0..8000).map(|n| format!("{n:05} {}", "\u{200b}".repeat(60))).collect();
        let reply = fit_lines(read(lines));
        let line = json_line(&reply);
        assert!(line.len() <= REPLY_MAX_BYTES, "{} bytes", line.len());
        let result: Read = serde_json::from_value(reply.result.expect("a result")).expect("a read");
        assert!(result.truncated && result.first_line > 0);
        assert!(result.lines.last().is_some_and(|last| last.starts_with("07999")), "the newest stay");
        assert_eq!(result.first_line as usize + result.lines.len(), 8000);
        let small = fit_lines(read(vec!["x".into()]));
        let small: Read = serde_json::from_value(small.result.expect("a result")).expect("a read");
        assert!(!small.truncated);
    }

    /// The estimate that decides whether a reply is measured never falls short of the reply, for the
    /// characters that grow most when written (a soft hyphen is two bytes and six as an escape).
    #[test]
    fn the_size_estimate_is_never_under_the_reply() {
        for ch in ['\u{ad}', '\u{200b}', 'a', '日', '\u{7f}', '\u{85}'] {
            let read = read(vec![ch.to_string().repeat(500); 4]);
            let wrote = json_line(&read).len();
            assert!(read.size_estimate() >= wrote, "{ch:?}: {} against {wrote}", read.size_estimate());
        }
    }

    #[test]
    fn blocks_over_the_cap_lose_the_output_of_the_oldest_first() {
        let block = |id: i64| BlockInfo {
            id,
            command: "x".into(),
            state: "finished".into(),
            exit_code: Some(0),
            took_ms: Some(1),
            first_line: 0,
            last_line: 1,
            output: Some("\u{200b}".repeat(60_000)),
            truncated: false,
        };
        let blocks = Blocks { terminal: 1, integration: true, blocks: (1..=8).map(block).collect() };
        let reply = fit_blocks(blocks);
        assert!(json_line(&reply).len() <= REPLY_MAX_BYTES);
        let result: Blocks = serde_json::from_value(reply.result.expect("a result")).expect("blocks");
        assert!(
            result.blocks[0].output.is_none() && result.blocks[0].truncated,
            "the oldest lost its output"
        );
        assert!(result.blocks[7].output.is_some(), "the newest kept it");
    }
}
