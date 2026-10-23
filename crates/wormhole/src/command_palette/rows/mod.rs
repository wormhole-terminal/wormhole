//! What the command palette lists, as data: what it knows when it opens (`Snapshot`), and
//! the rows that make of a scope and a query. Core's `search` finds and ranks; the settings come
//! from core's `settings_index` search, in its order. By file: `items` the things listed and the rows,
//! `snapshot` what the palette knows, `lists` what an empty query shows, `query` what a typed one finds
//! and `detail` the words under a terminal.

mod detail;
mod items;
mod lists;
mod query;
mod snapshot;
#[cfg(test)]
mod tests;

pub(crate) use detail::{question, terminal_detail};
pub(crate) use items::{
    FolderItem, Item, ProfileChoice, ProfileCommand, ProjectAction, ProjectItem, RECENT_MAX, Recent, Row,
    TerminalItem,
};
pub(crate) use snapshot::Snapshot;

use lists::{actions_by_section, recent_settings, suggestions, terminals_by_state};
use query::found;

use wormhole_core::view::search::Scope;

/// The palette's rows for `scope` and `query` (the words after any prefix).
pub(crate) fn rows(snapshot: &Snapshot, scope: Scope, query: &str) -> Vec<Row> {
    let query = query.trim();
    let rows = match (scope, query.is_empty()) {
        (Scope::All, true) => suggestions(snapshot),
        (Scope::Terminals, true) => terminals_by_state(snapshot),
        (Scope::Actions, true) => actions_by_section(snapshot),
        (Scope::Settings, true) => recent_settings(snapshot),
        _ => found(snapshot, scope, query),
    };
    if rows.is_empty() {
        return vec![Row::Note(if query.is_empty() { "Nothing here yet" } else { "Nothing matches" })];
    }
    rows
}

/// How many results a query found (the footer's count): every group's whole count.
pub(crate) fn result_count(rows: &[Row]) -> usize {
    rows.iter()
        .filter_map(|row| match row {
            Row::Label { count: Some(n), .. } => Some(*n),
            _ => None,
        })
        .sum()
}
