//! The terminal's text for Print and Export Text As: `wormhole-term`'s `contents`, as one call.

use crate::terminal::Terminal;

/// How much of the terminal's text a call returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ContentsScope {
    /// The live screen, one line per row (`screen_text`).
    Screen,
    /// The history and the screen. Soft-wrapped rows are joined, as a saved session has them; a
    /// full-screen program's alternate screen has no history, so it is the screen.
    Everything,
}

/// More lines than any terminal keeps (the history is capped well below), so nothing is cut.
const ALL_LINES: usize = (i32::MAX / 2) as usize;

#[uniffi::export]
impl Terminal {
    /// The text of `scope` as plain text; empty after `close`. Images' placeholder cells read as blanks.
    pub fn contents(&self, scope: ContentsScope) -> String {
        self.with_pty(|pty| match scope {
            ContentsScope::Screen => pty.screen_text(),
            ContentsScope::Everything => {
                pty.scrollback().text(ALL_LINES).unwrap_or_else(|| pty.screen_text())
            }
        })
        .unwrap_or_default()
    }

    /// What is typed in an AI CLI's input box on the screen (core's `prompt_in` heuristic: the last `>` or `❯`
    /// line and its box), for "Stash prompt". Nothing at a plain shell prompt, or after `close`.
    pub fn grab_prompt(&self) -> Option<String> {
        self.with_pty(|pty| pty.grab_prompt()).flatten()
    }
}
