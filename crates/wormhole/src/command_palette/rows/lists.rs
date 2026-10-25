//! The lists a query that is empty shows: Recent and the actions for here, the terminals by who needs input,
//! the actions by section, the settings opened lately.

use wormhole_core::keys::keymap::palette::{self as actions, Section as ActionSection};
use wormhole_core::notify::State;

use super::items::{Item, RECENT_MAX, Row, label, result, state_title};
use super::snapshot::Snapshot;

/// The empty query: Recent (what was opened from the palette last, terminals, folders,
/// settings and commits in one list), then the actions that make sense here, each with its key.
pub(super) fn suggestions(snapshot: &Snapshot) -> Vec<Row> {
    let recent: Vec<Row> = snapshot.recent_items().take(RECENT_MAX).map(result).collect();
    let mut rows = Vec::new();
    if !recent.is_empty() {
        rows.push(label("Recent", None));
        rows.extend(recent);
    }
    let suggested = actions::suggested(&snapshot.actions);
    if !suggested.is_empty() {
        rows.push(label("Actions", None));
        rows.extend(suggested.into_iter().map(|a| result(Item::Action(a.id))));
    }
    rows
}

/// The Terminals scope with nothing typed: every window's, under who needs input, working,
/// finished and idle.
pub(super) fn terminals_by_state(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows = Vec::new();
    let order = snapshot.terminals_by_urgency();
    for state in [State::NeedsYou, State::Working, State::Done, State::Idle] {
        let title = state_title(state);
        let tabs: Vec<i64> = order
            .iter()
            .map(|&ix| &snapshot.terminals[ix].terminal)
            .filter(|t| state_title(t.state) == title)
            .map(|t| t.tab)
            .collect();
        if tabs.is_empty() {
            continue;
        }
        rows.push(label(title, Some(tabs.len())));
        rows.extend(tabs.into_iter().map(|tab| result(Item::Terminal(tab))));
    }
    rows
}

/// The Actions scope with nothing typed: the table's actions in their groups (the shells to open
/// a terminal with at the end of Terminal's), the project's own (once trusted) between
/// View and App.
pub(super) fn actions_by_section(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows = action_section(snapshot, ActionSection::Terminal);
    if !rows.is_empty() {
        rows.extend(snapshot.shell_items().map(result));
    }
    rows.extend(action_section(snapshot, ActionSection::View));
    if let Some(project) = snapshot.project.as_ref().filter(|p| !p.actions.is_empty()) {
        rows.push(Row::Label { title: format!("Project · {}", project.name), count: None, trusted: true });
        rows.extend(project.actions.iter().map(|a| result(Item::ProjectAction(a.id.clone()))));
    }
    rows.extend(action_section(snapshot, ActionSection::App));
    let profiles = snapshot.profile_items();
    if !profiles.is_empty() {
        rows.push(label("Profiles", None));
        rows.extend(profiles.into_iter().map(result));
    }
    rows
}

pub(super) fn action_section(snapshot: &Snapshot, section: ActionSection) -> Vec<Row> {
    let actions: Vec<Row> = snapshot
        .actions
        .iter()
        .filter(|a| a.section == section)
        .map(|a| result(Item::Action(a.id)))
        .collect();
    if actions.is_empty() {
        return actions;
    }
    std::iter::once(label(section.title(), None)).chain(actions).collect()
}

/// The Settings scope with nothing typed: the settings opened from the palette lately, as
/// Recent lists them; typing finds any setting.
pub(super) fn recent_settings(snapshot: &Snapshot) -> Vec<Row> {
    let recent: Vec<Row> =
        snapshot.recent_items().filter(|item| matches!(item, Item::Setting(_))).map(result).collect();
    if recent.is_empty() {
        return vec![Row::Note("Type to find a setting")];
    }
    std::iter::once(label("Recent", None)).chain(recent).collect()
}
