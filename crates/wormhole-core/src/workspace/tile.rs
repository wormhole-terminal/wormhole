//! What a terminal's tile draws, and how loud it is: the glyph a row, a folded group's head, the
//! palette and the status bar show for a terminal, and the order of urgency they pick by. The rules
//! are the app's (`sidebar/row/tile.rs` `glyph_for`, `style/status.rs` `From<StatusGlyph>`), here as
//! data so a shell draws them as they are.

use crate::view::sidebar::Urgency;

use super::facts::TerminalFacts;

/// The mark on a terminal's tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGlyph {
    /// An agent (or a program reporting its status) works.
    Working,
    /// An agent asks something.
    NeedsYou,
    /// An agent finished and nobody has looked.
    Done,
    /// A command or an agent ended on an error nobody has looked at, or the last command failed.
    Failed,
    /// An agent waiting for a prompt.
    Idle,
    /// A terminal without an agent: a command runs in it, or it has output nobody has seen.
    Terminal { unread: bool },
    /// A shell at its prompt, or one that ended.
    Shell,
}

impl TileGlyph {
    /// How loud the tile is, for choosing which of several to show (core's order of urgency).
    pub fn urgency(self) -> Urgency {
        match self {
            Self::NeedsYou => Urgency::NeedsYou,
            Self::Failed => Urgency::Failed,
            Self::Working => Urgency::Working,
            Self::Terminal { unread: true } => Urgency::NewOutput,
            Self::Terminal { unread: false } => Urgency::Running,
            Self::Done => Urgency::Done,
            Self::Idle | Self::Shell => Urgency::Idle,
        }
    }
}

/// The glyph a terminal's facts are drawn with. A plain terminal's failure shows until the next
/// command starts (seen or not), in place of the quiet terminal and its output.
pub fn glyph_of(facts: &TerminalFacts) -> TileGlyph {
    use crate::view::sidebar::RowState;
    let state = facts.row_state();
    match (facts.told(), state) {
        (_, RowState::Exited) => TileGlyph::Shell,
        (true, RowState::Working) => TileGlyph::Working,
        (true, RowState::NeedsYou) => TileGlyph::NeedsYou,
        (true, RowState::Finished) => TileGlyph::Done,
        (_, RowState::Failed) => TileGlyph::Failed,
        (true, RowState::Idle) => TileGlyph::Idle,
        (false, _) if facts.command.is_some() => TileGlyph::Terminal { unread: false },
        (false, _) if facts.failure.is_some() => TileGlyph::Failed,
        (false, _) if facts.unread => TileGlyph::Terminal { unread: true },
        (false, _) => TileGlyph::Shell,
    }
}
