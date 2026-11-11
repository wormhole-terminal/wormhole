//! What a window asks of its workspace for the session hosts: which terminals are in front of the person (fed by their
//! hosts) and where one is, to start another in its place.

use crate::workspace::Workspace;

#[uniffi::export]
impl Workspace {
    /// The selected terminal and the panes shown with it (all of a split, or only its zoomed pane): a hosted terminal
    /// on screen is fed a repaint and then the stream, the others stay headless (`window/hosting.rs`).
    pub fn shown(&self) -> Vec<i64> {
        self.shared().peek_model(|model| model.shown_ids())
    }

    /// The folder terminal `id` is in, as its shell last said.
    pub fn cwd_of(&self, id: i64) -> Option<String> {
        self.shared().peek_model(|model| {
            model.facts(id).and_then(|facts| facts.cwd.as_ref()).map(|cwd| cwd.to_string_lossy().into_owned())
        })
    }
}
