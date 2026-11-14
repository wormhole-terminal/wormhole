//! The terminals as tabs (`sidebar/tab_bar.rs` in the GPUI app): the sets the bar shows, which one is current and
//! their alerts (`view::sidebar::tabs`, through `WorkspaceModel::tab_sets_view`), and what VoiceOver hears for each tab
//! of the open set, in the core's words (`view::sidebar::spoken::tab_label`): its name, its state, whether it is the
//! selected one, and its place among the set's tabs.

use wormhole_core::view::sidebar::{Urgency, spoken};
use wormhole_core::workspace::{TabSetView, TabSetsView};

use crate::workspace::Workspace;

#[uniffi::remote(Record)]
pub struct TabSetView {
    pub group: Option<u32>,
    pub label: String,
    pub members: Vec<i64>,
    pub alert: Option<Urgency>,
}

#[uniffi::remote(Record)]
pub struct TabSetsView {
    pub sets: Vec<TabSetView>,
    pub current: Option<u32>,
    pub others_alert: Option<Urgency>,
}

#[uniffi::export]
impl Workspace {
    /// The tab bar's sets: every group (a folded one with all its terminals), then the terminals in no group (`group`
    /// `None`, labelled "Ungrouped" in English: the window words it), the one the selected terminal is in, each set's
    /// loudest alert and the closed switcher's dot.
    pub fn tab_sets(&self) -> TabSetsView {
        self.shared().peek_model(|model| model.tab_sets_view())
    }

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
