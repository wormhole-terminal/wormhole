//! What the window tells those who watch it (`wh events`): a terminal's state, title and folder
//! changing, a command starting or ending, terminals and splits coming and going.
//!
//! Each publishing function asks `crate::automation` whether anyone listens for its topic before it
//! reads or builds anything, and none of them touches a terminal's output, so a window that nobody
//! watches does what it always did. The ids are the ones `list` shows: the window's, the terminal's
//! (a terminal and its tab and pane are the same number) and the split's.
//!
//! Part of the window: it uses `crate::automation` and core, never the app root that makes the
//! subscriptions.

use gpui_kit::*;
use wormhole_core::git::{ChangeKind, RepoStatus};
use wormhole_core::ipc::control::TerminalState;
use wormhole_core::ipc::control::event::{BlockPhase, Body, Event, Topic};
use wormhole_core::view::sidebar::SplitId;
use wormhole_term::{Block, BlockState};

use crate::automation::{publish, wants};
use crate::terminal_view::{CommandMark, TerminalSummary};
use crate::window::Workspace;
use crate::window::ambient::state_of;
use crate::window::control::agent_id;

/// A summary's state as the protocol speaks of it (what `list` would say).
fn state(summary: &TerminalSummary) -> TerminalState {
    state_of(summary.status, summary.source.told(), summary.foreground.as_ref()).into()
}

/// A command line is cut to this many characters in an event: it is a line of terminal text.
const COMMAND_CHARS: usize = 512;

impl Workspace {
    fn event(&self, terminal: Option<i64>, body: Body) -> Event {
        Event { window: Some(self.window_id), terminal, body }
    }

    /// Session `ix`'s summary moved from `old` to what it holds now: tell of a change of state, of
    /// title and of folder. `old` is only compared, so nothing is read unless somebody listens.
    pub(super) fn publish_summary_change(&self, ix: usize, old: &TerminalSummary, cx: &App) {
        // Before anything is told: what happens in a terminal that was just hidden is not.
        if old.agent_hidden != self.sessions[ix].pane.summary.agent_hidden {
            crate::automation::set_hidden(
                cx,
                self.sessions[ix].id,
                self.sessions[ix].pane.summary.agent_hidden,
            );
        }
        let (status, title, folder) =
            (wants(cx, Topic::Status), wants(cx, Topic::Title), wants(cx, Topic::Folder));
        if !(status || title || folder) {
            return;
        }
        let session = &self.sessions[ix];
        let (id, new) = (session.id, &session.pane.summary);
        if status {
            let (was, now) = (state(old), state(new));
            if was != now {
                publish(cx, Topic::Status, || {
                    let body = Body::Status {
                        status: now,
                        was,
                        agent: new.agent.and_then(agent_id),
                        message: new.agent_message.clone(),
                    };
                    self.event(Some(id), body)
                });
            }
        }
        if title && old.title != new.title {
            publish(cx, Topic::Title, || {
                let body = Body::Title { title: new.title.clone(), name: session.name().to_string() };
                self.event(Some(id), body)
            });
        }
        if folder && old.cwd != new.cwd {
            publish(cx, Topic::Folder, || {
                let folder = new.cwd.as_ref().map(|path| path.to_string_lossy().into_owned());
                self.event(Some(id), Body::Folder { folder })
            });
        }
    }

    /// What git says of a terminal's folder, as the bus tells it. Counts only: the files and their lines stay
    /// the panel's.
    pub(super) fn git_body(status: &RepoStatus) -> Body {
        let untracked = status.files.iter().filter(|file| file.kind == ChangeKind::Untracked).count();
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let upstream = status.upstream.as_ref();
        Body::Git {
            branch: status.branch.clone(),
            ahead: upstream.map_or(0, |up| up.ahead),
            behind: upstream.map_or(0, |up| up.behind),
            dirty: count(status.files.len() - untracked),
            untracked: count(untracked),
        }
    }

    /// What git said of session `ix`'s folder before a read that is on its way, when anyone listens: the
    /// answer to compare with ([`publish_git`](Self::publish_git)).
    pub(super) fn git_before(&self, ix: usize, cx: &App) -> Option<Body> {
        wants(cx, Topic::Git).then(|| self.sessions[ix].git.status.as_ref().map(Self::git_body)).flatten()
    }

    /// Session `ix`'s repository was read and the answer is not what it was (`before`): tell the new one.
    pub(super) fn publish_git(&self, ix: usize, before: Option<Body>, cx: &App) {
        if !wants(cx, Topic::Git) {
            return;
        }
        let session = &self.sessions[ix];
        let Some(status) = session.git.status.as_ref() else { return };
        let body = Self::git_body(status);
        if before.as_ref() != Some(&body) {
            publish(cx, Topic::Git, || self.event(Some(session.id), body));
        }
    }

    /// A terminal was opened here, in `folder` as it was asked for.
    pub(super) fn publish_terminal_opened(&self, id: i64, folder: Option<&str>, cx: &App) {
        publish(cx, Topic::Terminals, || {
            self.event(Some(id), Body::TerminalOpened { folder: folder.map(str::to_string) })
        });
    }

    /// A terminal is closed for good.
    pub(super) fn publish_terminal_closed(&self, id: i64, cx: &App) {
        publish(cx, Topic::Terminals, || self.event(Some(id), Body::TerminalClosed));
        crate::automation::forget_terminal(cx, id);
    }

    /// A terminal was named, or got the name of what runs in it again (none).
    pub(super) fn publish_renamed(&self, id: i64, name: Option<&str>, cx: &App) {
        publish(cx, Topic::Terminals, || {
            self.event(Some(id), Body::Renamed { name: name.map(str::to_string) })
        });
    }

    /// The bell rang in terminal `id`.
    pub(super) fn publish_bell(&self, id: i64, cx: &App) {
        publish(cx, Topic::Bell, || self.event(Some(id), Body::Bell));
    }

    /// Session `ix`'s program printed: note it for the output worker and wake the waiters on its
    /// screen, which look at the terminal off this thread. Nothing is read, built or drawn here; with
    /// nobody asking for this terminal nothing is noted either.
    pub(super) fn publish_printed(&self, ix: usize, cx: &App) {
        let session = &self.sessions[ix];
        let terminal = session.terminal().read(cx);
        crate::automation::printed(cx, self.window_id, session.id, || terminal.reader());
    }

    /// Terminal `id`'s shell marked a command's start or end: tell which command it was. The mark carries
    /// the number of its prompt, which is the block's id, so a mark heard late (the shell having marked
    /// others since) is still told about the right command; how it went comes from the shell's own log
    /// by that number, and the line typed from the grid when its rows are still there.
    pub(super) fn publish_command(&self, id: i64, mark: CommandMark, cx: &App) {
        if !wants(cx, Topic::Blocks) {
            return;
        }
        // Before any prompt (0) the log has nothing to say.
        let prompt = mark.prompt();
        let Some(session) = self.sessions.iter().find(|session| session.id == id) else { return };
        let terminal = session.terminal().read(cx);
        let Some(state) = terminal.command_state(prompt).filter(|_| prompt > 0) else { return };
        // An Enter on an empty line is marked as ending a block too, with no command in it: that is
        // not a command finishing, and the command before it already said so.
        if !state.ran() {
            return;
        }
        let (phase, exit_code) = match mark {
            CommandMark::Started { .. } => (BlockPhase::Started, None),
            CommandMark::Finished { code, .. } => (BlockPhase::Finished, code),
        };
        // How long it took is known once it ended, and only the end says it.
        let took_ms = match (phase, state) {
            (BlockPhase::Finished, BlockState::Finished { took, .. }) => {
                took.map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX))
            }
            _ => None,
        };
        publish(cx, Topic::Blocks, || {
            let block = terminal.block_numbered(prompt);
            let command =
                wormhole_term::clean(block.as_ref().map_or("", |block| &block.command), COMMAND_CHARS);
            // The rows of its output are known once it ended, from the marks on the grid.
            let output_lines =
                block.as_ref().filter(|_| phase == BlockPhase::Finished).map(Block::output_rows);
            let body = Body::Block {
                block: i64::try_from(prompt).unwrap_or(i64::MAX),
                phase,
                exit_code,
                took_ms,
                command: (!command.is_empty()).then_some(command),
                output_lines,
            };
            self.event(Some(id), body)
        });
    }

    /// The window is going: every split in it dissolves and every terminal in it closes, told before the
    /// window's own end so a watcher's picture of it empties in order.
    pub(crate) fn publish_teardown(&self, cx: &App) {
        if !wants(cx, Topic::Splits) && !wants(cx, Topic::Terminals) {
            return;
        }
        for split in &self.splits {
            self.publish_split_gone(split.id, cx);
        }
        for session in &self.sessions {
            self.publish_terminal_closed(session.id, cx);
        }
    }

    /// Split `split` is no longer in this window (it dissolved, or moved with its terminals).
    pub(super) fn publish_split_gone(&self, split: SplitId, cx: &App) {
        publish(cx, Topic::Splits, || {
            self.event(None, Body::Split { split, terminals: Vec::new(), focused: None })
        });
    }

    /// Every split of this window as it stands (a window that has just been given terminals running in
    /// splits).
    pub(crate) fn publish_all_splits(&self, cx: &App) {
        for split in &self.splits {
            self.publish_split(split.id, cx);
        }
    }

    /// A split was made or changed, or (with no members left in `self.splits`) is gone.
    pub(super) fn publish_split(&self, split: SplitId, cx: &App) {
        publish(cx, Topic::Splits, || {
            let layout = self.split_with(split).map(|split| &split.layout);
            let body = Body::Split {
                split,
                terminals: layout.map(|layout| layout.panes()).unwrap_or_default(),
                focused: layout.map(|layout| layout.focused()),
            };
            self.event(None, body)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::COMMAND_CHARS;

    #[test]
    fn a_command_in_an_event_is_one_cut_line() {
        let long = format!("echo {}\n\u{1b}[31m", "x".repeat(1000));
        let cleaned = wormhole_term::clean(&long, COMMAND_CHARS);
        assert_eq!(cleaned.chars().count(), COMMAND_CHARS + 1, "cut with an ellipsis");
        assert!(!cleaned.contains(['\n', '\u{1b}']));
    }
}
