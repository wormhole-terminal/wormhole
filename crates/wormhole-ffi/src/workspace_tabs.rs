//! The terminals as tabs (`sidebar/tab_bar.rs` in the GPUI app): what VoiceOver hears for each tab of the open set,
//! in the core's words (`view::sidebar::spoken::tab_label`): its name, its state, whether it is the selected one, and
//! its place among the set's tabs.

use wormhole_core::view::sidebar::spoken;

use crate::workspace::Workspace;

#[uniffi::export]
impl Workspace {
    /// The spoken label of each of `ids`, the tabs of one set in the bar's order ("build, working, selected, 2 of 4").
    /// A terminal the window no longer has gets an empty label; the answer is always as long as `ids`.
    pub fn tab_labels(&self, ids: Vec<i64>) -> Vec<String> {
        let terminals = self.shared().peek_model(|model| model.terminals_snapshot()).terminals;
        let of = ids.len();
        ids.iter()
            .enumerate()
            .map(|(at, id)| {
                terminals.iter().find(|terminal| terminal.id == *id).map_or_else(String::new, |terminal| {
                    spoken::tab_label(
                        &terminal.name,
                        spoken::state_word(terminal.state),
                        terminal.selected,
                        at + 1,
                        of,
                    )
                })
            })
            .collect()
    }
}
