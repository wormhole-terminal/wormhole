//! Which parts the terminal's title row shows, and in what order: the person's own arrangement
//! (`title_row` in `config.json`), as a toolbar's items are.
//!
//! The row is one list of parts with one flexible space in it: what comes before the space sits at
//! the row's left end, the rest at its right end. A part that is not in the list is hidden. The
//! space can be moved, never removed. Each part is still drawn only when there is something to show
//! (a branch needs a repository, Open in… a folder) and as the row's room allows
//! ([`super::title_row::fit_row`], whose order of giving way doesn't depend on where a part sits).

use serde::{Deserialize, Serialize};

use crate::view::reorder;

/// One part of the title row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitlePart {
    /// The terminal's folder, cut in its middle where it is long.
    Folder,
    /// The folder's branch, as a chip (and, in a submodule, whose it is).
    Branch,
    /// A dot while the tree has uncommitted changes, and the commits ahead and behind.
    BranchStatus,
    /// The terminal's status tile and its name.
    Terminal,
    /// How the terminal is doing ("working · 4m"), when the status bar doesn't say it.
    TerminalStatus,
    /// The flexible space between the row's two ends.
    Space,
    /// A coding agent's model and how full its context window is.
    AgentGauge,
    /// The ports the terminal's programs listen on.
    Ports,
    /// The branch's pull request and its checks.
    PullRequest,
    /// Open in…: the folder in an editor, and the menu of the others.
    OpenIn,
    /// The two split buttons.
    Splits,
}

/// How a part joins the parts beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    /// A step of the breadcrumb (folder › branch › terminal): a chevron separates two of them.
    Crumb,
    /// Words or marks that belong to the crumb before them, with no chevron of their own.
    Inline,
    /// A chip that stands on its own.
    Chip,
    /// An icon button; buttons side by side are flush.
    Button,
    Space,
}

impl TitlePart {
    /// Every part, in the default order.
    pub const ALL: [TitlePart; 11] = [
        TitlePart::Folder,
        TitlePart::Branch,
        TitlePart::BranchStatus,
        TitlePart::Terminal,
        TitlePart::TerminalStatus,
        TitlePart::Space,
        TitlePart::AgentGauge,
        TitlePart::Ports,
        TitlePart::PullRequest,
        TitlePart::OpenIn,
        TitlePart::Splits,
    ];

    pub fn kind(self) -> PartKind {
        match self {
            TitlePart::Folder | TitlePart::Branch | TitlePart::Terminal => PartKind::Crumb,
            TitlePart::BranchStatus | TitlePart::TerminalStatus => PartKind::Inline,
            TitlePart::AgentGauge | TitlePart::Ports | TitlePart::PullRequest => PartKind::Chip,
            TitlePart::OpenIn | TitlePart::Splits => PartKind::Button,
            TitlePart::Space => PartKind::Space,
        }
    }

    /// Whether the person can hide it: every part but the space.
    pub fn hideable(self) -> bool {
        self != TitlePart::Space
    }
}

/// The title row's parts as the person arranged them: each at most once, the space exactly once.
/// Saved as the list of their names. A list read back is made whole by [`TitleRow::new`], and a name
/// it doesn't know (a typo, or a part of a newer build) is left out rather than making the whole
/// settings file unreadable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TitleRow(Vec<TitlePart>);

impl<'de> Deserialize<'de> for TitleRow {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let names = Vec::<serde_json::Value>::deserialize(deserializer)?;
        Ok(Self::new(names.into_iter().filter_map(|name| {
            TitlePart::deserialize(&name)
                .inspect_err(|_| tracing::warn!("title_row: no part {name} in the title row, left out"))
                .ok()
        })))
    }
}

impl Default for TitleRow {
    fn default() -> Self {
        Self(TitlePart::ALL.to_vec())
    }
}

impl TitleRow {
    /// A row from any list (a hand-edited file): a part named twice keeps its first place, and a
    /// missing space goes where it is by default.
    pub fn new(parts: impl IntoIterator<Item = TitlePart>) -> Self {
        let mut row = Self(Vec::new());
        for part in parts {
            if !row.0.contains(&part) {
                row.0.push(part);
            }
        }
        row.show(TitlePart::Space);
        row
    }

    /// The default row without the parts whose switch is off: what a settings file from before the
    /// row could be arranged said with its four switches.
    pub fn from_switches(branch_status: bool, agent_gauge: bool, ports: bool, pull_request: bool) -> Self {
        let off = [
            (TitlePart::BranchStatus, branch_status),
            (TitlePart::AgentGauge, agent_gauge),
            (TitlePart::Ports, ports),
            (TitlePart::PullRequest, pull_request),
        ];
        Self::new(TitlePart::ALL.into_iter().filter(|part| !off.iter().any(|(p, on)| p == part && !on)))
    }

    /// The parts shown, in order, the space among them.
    pub fn parts(&self) -> &[TitlePart] {
        &self.0
    }

    pub fn shows(&self, part: TitlePart) -> bool {
        self.0.contains(&part)
    }

    /// The parts before the space (the row's left end) and after it (its right end).
    pub fn ends(&self) -> (&[TitlePart], &[TitlePart]) {
        match self.0.iter().position(|part| *part == TitlePart::Space) {
            Some(ix) => (&self.0[..ix], &self.0[ix + 1..]),
            None => (&self.0, &[]),
        }
    }

    /// The parts that are hidden, in the default order.
    pub fn hidden(&self) -> impl Iterator<Item = TitlePart> + '_ {
        TitlePart::ALL.into_iter().filter(|part| !self.shows(*part))
    }

    /// Show or hide `part`. A part shown again goes back where it is by default: right after the
    /// nearest part before it in the default order that is shown (the space counts), else first.
    /// The space can't be hidden. Returns whether anything changed.
    pub fn set_shown(&mut self, part: TitlePart, shown: bool) -> bool {
        match (shown, self.shows(part)) {
            (true, false) => {
                self.show(part);
                true
            }
            (false, true) if part.hideable() => {
                self.0.retain(|p| *p != part);
                true
            }
            _ => false,
        }
    }

    fn show(&mut self, part: TitlePart) {
        if self.shows(part) {
            return;
        }
        let place = TitlePart::ALL.iter().position(|p| *p == part).unwrap_or_default();
        let at = TitlePart::ALL[..place]
            .iter()
            .rev()
            .find_map(|p| self.0.iter().position(|shown| shown == p))
            .map_or(0, |ix| ix + 1);
        self.0.insert(at, part);
    }

    /// Move the part at `from` to `to` (an index into the row after the move, as
    /// [`reorder::move_item`] takes it). Returns whether anything moved.
    pub fn move_part(&mut self, from: usize, to: usize) -> bool {
        reorder::move_item(&mut self.0, from, to)
    }

    /// Whether this is the default arrangement.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// For the parts one end of the row draws, in order: whether a chevron goes before each. A crumb
/// gets one when the part drawn before it, past the inline marks that belong to it, is a crumb too:
/// `folder › branch ● › terminal`, while a chip between two crumbs parts them without one.
pub fn chevrons(drawn: &[TitlePart]) -> Vec<bool> {
    let mut last = None;
    drawn
        .iter()
        .map(|part| {
            let kind = part.kind();
            let chevron = kind == PartKind::Crumb && last == Some(PartKind::Crumb);
            if kind != PartKind::Inline {
                last = Some(kind);
            }
            chevron
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::TitlePart::*;
    use super::{TitlePart, TitleRow, chevrons};

    fn row(parts: &[TitlePart]) -> TitleRow {
        TitleRow::new(parts.iter().copied())
    }

    #[test]
    fn the_default_row_is_the_breadcrumb_then_the_extras_and_buttons_at_the_right() {
        let row = TitleRow::default();
        let (left, right) = row.ends();
        assert_eq!(left, [Folder, Branch, BranchStatus, Terminal, TerminalStatus]);
        assert_eq!(right, [AgentGauge, Ports, PullRequest, OpenIn, Splits]);
        assert!(row.is_default() && row.hidden().next().is_none());
    }

    #[test]
    fn a_hand_written_list_keeps_each_part_once_and_always_has_its_space() {
        assert_eq!(row(&[Splits, Folder, Splits]).parts(), [Splits, Folder, Space]);
        assert_eq!(row(&[Space, Folder, Space]).parts(), [Space, Folder]);
        // No space: it goes after the last part shown that comes before it by default.
        assert_eq!(row(&[Folder, OpenIn, Terminal]).parts(), [Folder, OpenIn, Terminal, Space]);
        assert_eq!(row(&[Ports, OpenIn]).parts(), [Space, Ports, OpenIn]);
        assert_eq!(row(&[]).parts(), [Space]);
    }

    #[test]
    fn the_old_switches_hide_their_parts() {
        let row = TitleRow::from_switches(true, false, true, false);
        assert!(row.shows(BranchStatus) && row.shows(Ports));
        assert!(!row.shows(AgentGauge) && !row.shows(PullRequest));
        assert_eq!(row.hidden().collect::<Vec<_>>(), [AgentGauge, PullRequest]);
        assert!(TitleRow::from_switches(true, true, true, true).is_default());
    }

    #[test]
    fn a_part_shown_again_goes_back_beside_its_default_neighbour() {
        let mut row = row(&[Splits, Folder, Terminal, Space, OpenIn]);
        assert!(row.set_shown(Branch, true));
        assert_eq!(row.parts(), [Splits, Folder, Branch, Terminal, Space, OpenIn]);
        assert!(row.set_shown(Ports, true), "after the space, its nearest shown part before it");
        assert_eq!(row.parts(), [Splits, Folder, Branch, Terminal, Space, Ports, OpenIn]);
        assert!(!row.set_shown(Ports, true), "already shown");
        // Nothing before it is shown: first.
        let mut bare = TitleRow::new([Space, Splits]);
        bare.set_shown(Folder, true);
        assert_eq!(bare.parts(), [Folder, Space, Splits]);
    }

    #[test]
    fn every_part_but_the_space_can_be_hidden() {
        let mut row = TitleRow::default();
        for part in TitlePart::ALL {
            row.set_shown(part, false);
        }
        assert_eq!(row.parts(), [Space]);
        assert!(!row.set_shown(Space, false));
        assert_eq!(row.ends(), (&[][..], &[][..]));
    }

    #[test]
    fn parts_move_and_the_space_moves_with_them() {
        let mut row = TitleRow::default();
        // The splits to the front, the space to the very end: everything at the left.
        assert!(row.move_part(10, 0));
        assert!(row.move_part(6, 10));
        assert_eq!(row.parts()[0], Splits);
        let (left, right) = row.ends();
        assert_eq!((left.len(), right.len()), (10, 0));
        assert!(!row.move_part(3, 3) && !row.move_part(11, 0));
    }

    #[test]
    fn it_is_saved_as_a_list_of_names() {
        let json = serde_json::to_string(&row(&[Folder, Space, AgentGauge, OpenIn])).expect("serializes");
        assert_eq!(json, r#"["folder","space","agent_gauge","open_in"]"#);
        let back: TitleRow =
            serde_json::from_str(r#"["terminal_status","pull_request","space"]"#).expect("deserializes");
        assert_eq!(back.parts(), [TerminalStatus, PullRequest, Space]);
        let mended: TitleRow = serde_json::from_str(r#"["ports","ports"]"#).expect("deserializes");
        assert_eq!(mended.parts(), [Space, Ports], "read back whole");
        let unknown: TitleRow =
            serde_json::from_str(r#"["folder","porst",3,"space"]"#).expect("still deserializes");
        assert_eq!(unknown.parts(), [Folder, Space], "what it can't read is left out");
    }

    #[test]
    fn chevrons_join_crumbs_across_their_inline_marks_but_not_across_a_chip() {
        assert_eq!(
            chevrons(&[Folder, Branch, BranchStatus, Terminal, TerminalStatus]),
            [false, true, false, true, false]
        );
        assert_eq!(chevrons(&[Terminal, Folder]), [false, true]);
        assert_eq!(chevrons(&[Folder, Ports, Branch]), [false, false, false]);
        assert_eq!(chevrons(&[BranchStatus, Folder]), [false, false], "a mark first joins nothing");
        assert_eq!(chevrons(&[OpenIn, Folder, Splits, Terminal]), [false, false, false, false]);
    }
}
