//! A terminal's own folder as the all-terminals snapshot hands it to Swift: `cwd` is the full path as a key of
//! `files_panel::names` where its name is not text, so the panel's tabs (Files, Changes, History, Project) and the notes
//! sync read the very folder the terminal is in. The `~`-shortened `folder` beside it is for drawing.

use wormhole_core::workspace::{TerminalsSnapshot, WorkspaceModel};

use crate::files_panel::names;

/// `snapshot` with each terminal's `cwd` as a key of its bytes.
pub(crate) fn keyed(model: &WorkspaceModel, mut snapshot: TerminalsSnapshot) -> TerminalsSnapshot {
    for terminal in &mut snapshot.terminals {
        terminal.cwd = model.facts(terminal.id).and_then(|facts| facts.cwd.as_deref()).map(names::path_key);
    }
    snapshot
}
