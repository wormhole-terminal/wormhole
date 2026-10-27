//! `run`'s half in the window: what a command typed at a prompt needs to know first, and typing it.

use gpui_kit::*;

use crate::window::Workspace;
use crate::window::feature_terminals::paste_text;

/// ⌃U: the shell clears the line it has so far, as Put Command at Prompt does before it types.
const KILL_LINE: u8 = 0x15;

impl Workspace {
    /// The newest block that had run in `tab`'s terminal: the command typed now runs in a block after it.
    /// `None` outside the outer option: the terminal can't be reached; inside it: no command has run.
    pub(crate) fn control_last_block(&self, tab: i64, cx: &App) -> Option<Option<u64>> {
        let terminal = self.pane_terminal(tab)?;
        Some(terminal.read(cx).newest_block().map(|block| block.id))
    }

    /// Clear the line at `tab`'s prompt and type `command` there as a paste, pressing Return a moment
    /// after it (the shell reads the paste first). Whether the terminal was there. In a shell whose line
    /// editor is in vi's command mode ⌃U is not "clear the line", and the command is typed as that mode
    /// reads it: the documented price of `run` (`docs/cli.md`).
    pub(crate) fn control_run(&mut self, tab: i64, command: &str, cx: &mut Context<Self>) -> bool {
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        terminal.update(cx, |terminal, _| terminal.write_input(vec![KILL_LINE]));
        paste_text(&terminal, command, true, cx);
        true
    }
}
