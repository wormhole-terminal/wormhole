//! The model's tests: what the app's window and sidebar tests check, with no window and no child
//! process. Each file takes one concern; this one holds what they share.

mod chrome;
mod close;
mod drops;
mod feed;
mod fences;
mod folded;
mod group_actions;
mod groups;
mod listing;
mod splits;
mod status;
mod windows;
mod words;

use super::{Action, Event, ModelConfig, SidebarSnapshot, WorkspaceModel};

/// A model with `n` terminals, the first selected (as a window opens), ids 1 to `n`.
pub(super) fn model(n: usize) -> WorkspaceModel {
    let mut model = WorkspaceModel::new(ModelConfig {
        numbers: crate::view::sidebar::SidebarNumbers::List,
        ..ModelConfig::default()
    });
    for _ in 0..n {
        model.apply(Action::NewTerminal);
    }
    if n > 0 {
        model.apply(Action::Select(1));
    }
    model
}

/// The list as text: a group's head as `[name]`, a row as `id` or `id:number`, a folded head with
/// `+` and the rows it hides left out, as the sidebar shows them top to bottom.
pub(super) fn shape(snapshot: &SidebarSnapshot) -> Vec<String> {
    let mut lines = Vec::new();
    for section in &snapshot.sections {
        if let Some(head) = &section.head {
            lines.push(format!("[{}]{}", head.name, if head.folded { "+" } else { "" }));
        }
        for row in &section.rows {
            lines.push(match row.number {
                Some(n) => format!("{}:{n}", row.id),
                None => row.id.to_string(),
            });
        }
    }
    lines
}

pub(super) fn listed(model: &WorkspaceModel) -> Vec<String> {
    shape(&model.sidebar_snapshot(0))
}

pub(super) fn refused(events: &[Event]) -> bool {
    events.iter().any(|event| matches!(event, Event::Refused(_)))
}
