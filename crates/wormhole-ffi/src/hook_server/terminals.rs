//! The terminals as the control line reaches them: which one a target names (and whether the caller may
//! know of it), the facts the typing rules need, and keys, pastes and pointer events encoded for the program's
//! modes as they are when each goes out (the GPUI app's `terminal_view/control_input.rs` over the PTY).

use std::sync::Arc;

use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::keys::Stroke;
use wormhole_core::ipc::control::mouse::ButtonName;
use wormhole_core::ipc::control::{MouseOp, Target, TerminalState};
use wormhole_term::input::mouse::{self, LocatorPointer, MouseButton};
use wormhole_term::{KeySent, Pty, ScreenReader};

use super::core::Core;
use super::views::state_of;
use crate::terminal::Terminal;

/// What the typing rules need of a terminal.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabFacts {
    pub state: TerminalState,
    pub shell: Option<u32>,
    /// Its `needs_you` is a told question (an agent's or a program's own report), not a bell.
    pub asked: bool,
}

/// What became of a pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseSent {
    Sent,
    /// The program did not ask for the mouse, or not for this event.
    Unwanted,
    /// The row or column is outside the grid, which is this big.
    OutOfGrid {
        rows: usize,
        cols: usize,
    },
}

impl Terminal {
    /// The shell's process id, while it runs: whoever runs under it is reading its own terminal.
    pub(crate) fn shell_pid(&self) -> Option<u32> {
        self.with_pty(Pty::shell_pid).flatten()
    }

    /// A handle that reads this terminal's screen from any thread, while its shell runs.
    pub(crate) fn reader(&self) -> Option<ScreenReader> {
        self.with_pty(Pty::reader)
    }

    /// Press `strokes`, in order, each encoded with the modes the program has when it goes out; how many
    /// were sent (a stroke the modes make mean nothing is not).
    pub(crate) fn send_strokes(&self, strokes: &[Stroke]) -> usize {
        self.with_pty(|pty| {
            strokes
                .iter()
                .filter(|stroke| {
                    let key =
                        pty.send_key(|modes| Some(stroke.encode(modes)).filter(|bytes| !bytes.is_empty()));
                    matches!(key, KeySent::Sent { .. })
                })
                .count()
        })
        .unwrap_or(0)
    }

    /// The control line typed into this terminal: typing as far as the terminal goes (GPUI's `write_input` and
    /// `send_strokes`). What the program drew as its own selection is forgotten (a paste no longer replaces it), and the
    /// view hears it (`TermEvent::ControlTyped`). A pointer event is not typing and does neither.
    pub(crate) fn control_typed(&self) {
        self.interaction.app_selection().forget();
        self.events.push_event(crate::events::TermEvent::ControlTyped);
    }

    /// Send `op` to a program that asked for the mouse, at a row and column of the grid.
    pub(crate) fn send_mouse(&self, op: &MouseOp) -> MouseSent {
        self.with_pty(|pty| {
            let size = pty.size();
            let (rows, cols) = (usize::from(size.rows), usize::from(size.cols));
            let (row, col) = op.at();
            if row as usize >= rows || col as usize >= cols {
                return MouseSent::OutOfGrid { rows, cols };
            }
            let modes = pty.mode();
            if mouse::wants_mouse(modes) {
                let cell = (size.cell_width, size.cell_height);
                return if pty.write_encoded(|modes| op.encode(modes, cell)) {
                    MouseSent::Sent
                } else {
                    MouseSent::Unwanted
                };
            }
            if !mouse::wants_locator(modes) || op.is_wheel() {
                return MouseSent::Unwanted;
            }
            let at = LocatorPointer {
                col: col as usize,
                row: row as usize,
                x: (col as f32 * size.cell_width + size.cell_width / 2.0) as u32,
                y: (row as f32 * size.cell_height + size.cell_height / 2.0) as u32,
            };
            let button = |name: Option<ButtonName>| match name.unwrap_or(ButtonName::Left) {
                ButtonName::Left => MouseButton::Left,
                ButtonName::Middle => MouseButton::Middle,
                ButtonName::Right => MouseButton::Right,
            };
            match op {
                MouseOp::Click(p) => {
                    for _ in 0..p.count.max(1) {
                        pty.locator(Some((button(p.button), true)), at);
                        pty.locator(Some((button(p.button), false)), at);
                    }
                }
                MouseOp::Press(p) => pty.locator(Some((button(p.button), true)), at),
                MouseOp::Release(p) => pty.locator(Some((button(p.button), false)), at),
                MouseOp::Move(_) => pty.locator(None, at),
                MouseOp::Scroll(_) => return MouseSent::Unwanted,
            }
            MouseSent::Sent
        })
        .unwrap_or(MouseSent::Unwanted)
    }
}

impl Core {
    /// Terminal `id`'s registered `Terminal`.
    pub(crate) fn terminal(&self, id: i64) -> Option<Arc<Terminal>> {
        self.shares().iter().find_map(|shared| shared.terminal_of(id))
    }

    /// The terminal `target` names (one id whichever way it is named), unless it is hidden from agents and
    /// the caller does not run inside it (by its place in the process tree, never by what it says): then it
    /// is not found, as if it did not exist. The one lookup every request that names a terminal goes through.
    pub(crate) fn find(&self, target: Target, peer: &Peer) -> Option<i64> {
        let id = target.id();
        let hidden = self.owner_of(id)?.peek_model(|model| {
            model.terminals_snapshot().terminals.iter().find(|t| t.id == id).map(|t| t.agent_hidden)
        })?;
        let inside = self.terminal(id).and_then(|t| t.shell_pid()).is_some_and(|shell| peer.runs_in(shell));
        (!hidden || inside).then_some(id)
    }

    /// The terminals whose shell the caller runs under: the ones it is inside of.
    pub(crate) fn terminals_under(&self, peer: &Peer) -> Vec<i64> {
        self.registered()
            .into_iter()
            .filter(|(_, terminal)| terminal.shell_pid().is_some_and(|shell| peer.runs_in(shell)))
            .map(|(id, _)| id)
            .collect()
    }

    /// Every terminal `peer` may know of: those not hidden from agents, and the one it runs inside of.
    pub(crate) fn visible_ids(&self, peer: &Peer) -> Vec<i64> {
        let shares = self.shares();
        let hidden: Vec<i64> = shares
            .iter()
            .flat_map(|shared| {
                shared.peek_model(|model| {
                    model
                        .terminals_snapshot()
                        .terminals
                        .iter()
                        .filter(|t| t.agent_hidden)
                        .map(|t| t.id)
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let inside = self.terminals_under(peer);
        shares
            .iter()
            .flat_map(|shared| shared.peek_model(|model| model.ids()))
            .filter(|id| !hidden.contains(id) || inside.contains(id))
            .collect()
    }

    /// The pids of the shells of the terminals: where the process chain of a caller stops being an agent's
    /// and is a person's terminal.
    pub(crate) fn shell_pids(&self) -> Vec<u32> {
        self.registered().into_iter().filter_map(|(_, terminal)| terminal.shell_pid()).collect()
    }

    pub(crate) fn registered(&self) -> Vec<(i64, Arc<Terminal>)> {
        let ids: Vec<i64> =
            self.shares().iter().flat_map(|shared| shared.peek_model(|model| model.ids())).collect();
        ids.into_iter().filter_map(|id| Some((id, self.terminal(id)?))).collect()
    }

    /// A tab's state, the pid of its shell (none once it has ended) and whether its `needs_you` is a told
    /// question.
    pub(crate) fn tab_facts(&self, tab: i64) -> Option<TabFacts> {
        let (state, asked) = self
            .owner_of(tab)?
            .peek_model(|model| model.facts(tab).map(|facts| (state_of(facts.row_state()), facts.told())))?;
        Some(TabFacts { state, shell: self.terminal(tab).and_then(|t| t.shell_pid()), asked })
    }
}
