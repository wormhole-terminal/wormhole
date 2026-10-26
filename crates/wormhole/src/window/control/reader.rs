//! What `read`, `blocks` and the output events need of a terminal: a handle that reads its screen
//! from another thread, and the facts around it that only the window knows.

use gpui_kit::*;
use wormhole_term::ScreenReader;

use crate::window::Workspace;

/// A terminal as the control line reads it.
pub(crate) struct ReaderFacts {
    /// The terminal's id (its session's).
    pub terminal: i64,
    pub reader: ScreenReader,
    /// The title the program set, when it set one.
    pub title: Option<String>,
    /// The shell's process id: whoever runs under it is reading its own terminal.
    pub shell: Option<u32>,
}

impl Workspace {
    /// Every terminal of this window that runs a shell, as a reader: what a new stream of `output`
    /// starts watching from.
    pub(crate) fn control_readers(&self, cx: &App) -> Vec<ReaderFacts> {
        self.sessions
            .iter()
            .filter_map(|session| {
                let terminal = session.terminal().read(cx);
                Some(ReaderFacts {
                    terminal: session.id,
                    reader: terminal.reader()?,
                    title: None,
                    shell: terminal.shell_pid(),
                })
            })
            .collect()
    }

    /// The reader of `tab`'s terminal (a tab and its terminal are the same number), while its shell
    /// runs.
    pub(crate) fn control_reader(&self, tab: i64, cx: &App) -> Option<ReaderFacts> {
        let terminal = self.pane_terminal(tab)?;
        let terminal = terminal.read(cx);
        Some(ReaderFacts {
            terminal: tab,
            reader: terminal.reader()?,
            title: terminal.title.clone(),
            shell: terminal.shell_pid(),
        })
    }
}
