//! Every terminal of the window as a record, folded groups included: what the palette lists, the
//! status bar counts and a menu of "Show beside" or "Move to group" offers. The sidebar's snapshot
//! lists only the rows the list shows, so a terminal folded away would be lost to all of these.

use crate::view::sidebar::wording::display_path;
use crate::view::sidebar::{Entry, RowState};

use super::super::model::WorkspaceModel;
use super::super::session::Session;
use super::super::tile::{TileGlyph, glyph_of};
use super::AgentInfo;

/// One terminal, whether the list shows its row or a folded group hides it.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalSummary {
    pub id: i64,
    pub name: String,
    /// ⌘1 to ⌘9; a terminal folded away has none.
    pub number: Option<u32>,
    pub selected: bool,
    pub pinned: bool,
    pub state: RowState,
    pub glyph: TileGlyph,
    pub agent: Option<AgentInfo>,
    pub message: Option<String>,
    pub branch: Option<String>,
    /// The folder as the list shows it (`~/Projects/app`), and as a full path.
    pub folder: Option<String>,
    pub cwd: Option<String>,
    pub command: Option<String>,
    /// The group it is in, with its name.
    pub group: Option<u32>,
    pub group_name: Option<String>,
    /// The split it is a member of, when it has company.
    pub split: Option<i64>,
    /// Its group is folded: no row is listed for it.
    pub folded: bool,
    pub agent_hidden: bool,
    pub forget_text: bool,
    pub unread: bool,
    pub bell: bool,
}

/// How many terminals are in each state: the status bar's and the menu bar's counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalCounts {
    pub needs_you: u32,
    pub working: u32,
    pub finished: u32,
    pub failed: u32,
    /// Idle or ended.
    pub quiet: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalsSnapshot {
    pub revision: u64,
    pub selected: Option<i64>,
    /// In the order the list has them: groups' terminals folded away among them.
    pub terminals: Vec<TerminalSummary>,
    pub counts: TerminalCounts,
}

impl WorkspaceModel {
    /// Every terminal in the list's order, whatever is folded or searched for.
    pub fn terminals_snapshot(&self) -> TerminalsSnapshot {
        let listing = self.listing();
        let mut numbers = vec![None; self.sessions.len()];
        let mut order: Vec<usize> = Vec::with_capacity(self.sessions.len());
        for entry in &listing.entries {
            match *entry {
                // A group's terminals, those under a folded head included.
                Entry::Band { band, .. } => order.extend(listing.bands[band].members.iter().copied()),
                Entry::Row { session, band, number, .. } => {
                    numbers[session] = number;
                    if band.is_none() {
                        order.push(session);
                    }
                }
            }
        }
        let mut counts = TerminalCounts::default();
        let terminals = order
            .into_iter()
            .map(|ix| {
                let session = &self.sessions[ix];
                match session.facts.row_state() {
                    RowState::NeedsYou => counts.needs_you += 1,
                    RowState::Working => counts.working += 1,
                    RowState::Finished => counts.finished += 1,
                    RowState::Failed => counts.failed += 1,
                    RowState::Idle | RowState::Exited => counts.quiet += 1,
                }
                self.summary(session, numbers[ix])
            })
            .collect();
        TerminalsSnapshot { revision: self.revisions().terminals, selected: self.selected, terminals, counts }
    }

    fn summary(&self, session: &Session, number: Option<usize>) -> TerminalSummary {
        let facts = &session.facts;
        let group = self.sidebar.groups.group_of(session.id).and_then(|id| self.sidebar.groups.get(id));
        TerminalSummary {
            id: session.id,
            name: session.name(),
            number: number.map(|n| n as u32),
            selected: self.selected == Some(session.id),
            pinned: session.pinned,
            state: facts.row_state(),
            glyph: glyph_of(facts),
            agent: facts.agent.map(super::sidebar::agent_info),
            message: facts.message.clone(),
            branch: facts.branch.clone(),
            folder: facts.cwd.as_deref().map(|cwd| display_path(cwd, self.cfg.home.as_deref())),
            cwd: facts.cwd.as_ref().map(|cwd| cwd.to_string_lossy().into_owned()),
            command: facts.command.clone(),
            group: group.map(|group| group.id),
            group_name: group.map(|group| group.name.clone()),
            split: session.split.filter(|&id| self.members(id).len() > 1),
            folded: group.is_some_and(|group| self.sidebar.is_group_folded(group.id)),
            agent_hidden: session.agent_hidden,
            forget_text: session.forget_text,
            unread: facts.unread,
            bell: facts.bell,
        }
    }
}
