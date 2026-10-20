//! OSC 133: FinalTerm's shell marks, which divide the screen into blocks of prompt, command and
//! output.

use crate::grid::Content;

use crate::terminal::Terminal;
use crate::terminal::event::{Event, PromptKind, ShellMark};

impl Terminal {
    /// OSC 133: `A` prompt, `B` command line, `C` output, `D` finished. The pen carries the mark
    /// so every cell printed afterwards belongs to the block; the rows learn their block from it.
    pub(in crate::terminal) fn osc_shell_mark(&mut self, rest: &[u8]) {
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
