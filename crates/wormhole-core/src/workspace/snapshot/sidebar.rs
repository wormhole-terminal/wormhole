//! The sidebar as a record: core's `Listing` (order, pins, groups, folds, numbers, split marks) cut
//! into sections, each row with its finished words.

use std::path::Path;
use std::time::Duration;

use crate::view::sidebar::spoken::{self, Place};
use crate::view::sidebar::wording::display_path;
use crate::view::sidebar::{Entry, Listing, RowState, SplitMarks, Urgency, hints};

use super::super::facts::Millis;
use super::super::model::WorkspaceModel;
use super::super::session::Session;
use super::super::words;
use super::{AgentInfo, GroupHead, SidebarRow, SidebarSection, SidebarSnapshot, SplitInfo};

impl WorkspaceModel {
    /// The list top to bottom as sections of rows, with the words as they read at `now`. While a
    /// search is on, rows that do not match and sections left empty are not listed; the ⌘ numbers stay
    /// with their terminals, as they select.
    pub fn sidebar_snapshot(&self, now: Millis) -> SidebarSnapshot {
        let listing = self.listing();
        let query = self.chrome.search.trim().to_lowercase();
        let searching = !query.is_empty();
        let matching = |ix: usize| !searching || self.matches(&self.sessions[ix], &query);
        let mut sections: Vec<SidebarSection> = Vec::new();
        for entry in &listing.entries {
            match *entry {
                Entry::Band { band, position, of } => {
                    sections.push(self.group_section(&listing, band, position, of, &matching));
                }
                Entry::Row { session, band, position, of, number, marks } => {
                    if !matching(session) {
                        continue;
                    }
                    let row = self.row(session, now, (position, of), number, marks);
                    match (band, sections.last_mut()) {
                        (Some(_), Some(section)) if section.head.is_some() => section.rows.push(row),
                        (None, Some(section)) if section.head.is_none() => section.rows.push(row),
                        _ => sections.push(loose_section(row)),
                    }
                }
            }
        }
        sections.retain(|section| !section.rows.is_empty() || (section.head.is_some() && !searching));
        for section in sections.iter_mut().filter(|section| section.head.is_none()) {
            section.terminal_count = section.rows.len() as u32;
            section.alert_count = section.rows.iter().filter(|row| is_alert(row.state)).count() as u32;
        }
        SidebarSnapshot {
            revision: self.revisions().sidebar,
            selected: self.selected,
            sections,
            terminal_count: self.sessions.len() as u32,
            refresh_in_ms: self.refresh_in(now).map(|wait| wait.as_millis() as u64),
        }
    }

    /// Every word of the query is found in the terminal's name, folder, branch, command, agent or
    /// message (case does not matter), as the quick panel's search asks.
    fn matches(&self, session: &Session, query: &str) -> bool {
        let facts = &session.facts;
        let folder = facts.cwd.as_deref().map(|cwd| display_path(cwd, self.cfg.home.as_deref()));
        let agent = facts.agent.map(|id| crate::agents::agent(id).name.to_string());
        let haystack = [
            Some(session.name()),
            folder,
            facts.branch.clone(),
            facts.command.clone(),
            agent,
            facts.message.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
        query.split_whitespace().all(|word| haystack.contains(word))
    }

    fn group_section(
        &self,
        listing: &Listing,
        band: usize,
        position: usize,
        of: usize,
        matching: &dyn Fn(usize) -> bool,
    ) -> SidebarSection {
        let band = &listing.bands[band];
        let group = self.sidebar.groups.get(band.group);
        let passing: Vec<usize> = band.members.iter().copied().filter(|&ix| matching(ix)).collect();
        let alerts = passing.iter().filter(|&&ix| is_alert(self.sessions[ix].facts.row_state())).count();
        let folder = group
            .and_then(|group| group.folder.as_deref())
            .map(|folder| display_path(Path::new(folder), self.cfg.home.as_deref()));
        let color = group.map(|group| group.color.clone()).unwrap_or_default();
        let spoken = spoken::band_label(
            &band.label,
            band.pinned,
            Some(color.as_str()).filter(|color| !color.is_empty()),
            folder.as_deref(),
            passing.len(),
            alerts,
            !band.folded,
            position,
            of,
        );
        SidebarSection {
            key: band.key(),
            head: Some(GroupHead {
                id: band.group,
                name: band.label.clone(),
                color,
                pinned: band.pinned,
                folded: band.folded,
                folder,
                position: position as u32,
                of: of as u32,
                spoken,
            }),
            rows: Vec::new(),
            terminal_count: passing.len() as u32,
            alert_count: alerts as u32,
        }
    }

    fn row(
        &self,
        ix: usize,
        now: Millis,
        (position, of): (usize, usize),
        number: Option<usize>,
        marks: SplitMarks,
    ) -> SidebarRow {
        let session = &self.sessions[ix];
        let facts = &session.facts;
        let name = session.name();
        let selected = self.selected == Some(session.id);
        let place = Place { selected, position, of, number };
        let spoken = words::row_label(&name, facts, now, place, marks);
        SidebarRow {
            id: session.id,
            number: number.map(|n| n as u32),
            keycap: number.map(hints::keycap),
            selected,
            pinned: session.pinned,
            state: facts.row_state(),
            agent: facts.agent.map(|id| {
                let agent = crate::agents::agent(id);
                AgentInfo {
                    key: agent.command.to_string(),
                    name: agent.name.to_string(),
                    short_name: agent.short_name().to_string(),
                }
            }),
            message: facts.message.clone(),
            branch: facts.branch.clone(),
            folder: facts.cwd.as_deref().map(|cwd| display_path(cwd, self.cfg.home.as_deref())),
            unread: facts.unread,
            bell: facts.bell,
            failure_code: facts.failure.as_ref().map(|failure| failure.code),
            changes: facts.changes.map(|n| n as u32),
            notes: facts.notes as u32,
            second_line: words::second_line(facts, now, self.cfg.home.as_deref()),
            split: SplitInfo {
                id: session.split.filter(|_| marks.members > 1),
                bracket: marks.bracket,
                chain: marks.chain,
                on_screen: marks.on_screen,
                members: marks.members as u32,
            },
            position: position as u32,
            of: of as u32,
            spoken,
            name,
        }
    }

    /// The soonest a row's words that move with time read differently.
    fn refresh_in(&self, now: Millis) -> Option<Duration> {
        self.sessions.iter().filter_map(|session| words::next_change(&session.facts, now)).min()
    }
}

fn is_alert(state: RowState) -> bool {
    Urgency::from(state).is_alert()
}

fn loose_section(row: SidebarRow) -> SidebarSection {
    SidebarSection {
        key: format!("loose-{}", row.id),
        head: None,
        rows: vec![row],
        terminal_count: 0,
        alert_count: 0,
    }
}
