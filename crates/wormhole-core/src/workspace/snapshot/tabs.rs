//! The tab bar's sets as a record (`view::sidebar::tabs`, the GPUI app's `sidebar/tab_bar.rs`): every group, a folded
//! one with all its terminals, then the terminals in no group; the set the selected terminal is in; each set's loudest
//! alert and the one the closed switcher's dot shows. A shell draws them and words "Ungrouped" in its language.

use crate::view::sidebar::Urgency;
use crate::view::sidebar::tabs::{self, TabSet};

use super::super::model::WorkspaceModel;
use super::super::tile::glyph_of;

/// One set of tabs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSetView {
    /// The group it is; `None` for the terminals in no group (labelled `tabs::UNGROUPED`).
    pub group: Option<u32>,
    pub label: String,
    /// Its terminals' ids in the list's order, folded ones too: the tab bar has no folds.
    pub members: Vec<i64>,
    /// The loudest of its terminals' alerts (needs you before failed), if one has any.
    pub alert: Option<Urgency>,
}

/// The sets, which one is current, and the dot on the closed switcher.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabSetsView {
    pub sets: Vec<TabSetView>,
    /// The set the selected terminal is in.
    pub current: Option<u32>,
    /// The loudest alert of the sets other than the current one (the first when none is), when one needs you or failed:
    /// the amber dot (`others_need_you`).
    pub others_alert: Option<Urgency>,
}

impl WorkspaceModel {
    pub fn tab_sets_view(&self) -> TabSetsView {
        let listing = self.listing();
        let sets = tabs::tab_sets(&listing);
        let urgency = |ix: usize| glyph_of(&self.sessions[ix].facts).urgency();
        let loudest = |set: &TabSet| set.members.iter().map(|&ix| urgency(ix)).min().filter(|u| u.is_alert());
        let current = self.selected_index().and_then(|ix| tabs::current_set(&sets, ix));
        let at = current.unwrap_or(0);
        let others_alert = tabs::others_need_you(&sets, at, &urgency)
            .then(|| {
                sets.iter().enumerate().filter(|&(ix, _)| ix != at).filter_map(|(_, set)| loudest(set)).min()
            })
            .flatten();
        TabSetsView {
            sets: sets
                .iter()
                .map(|set| TabSetView {
                    group: set.band.map(|band| listing.bands[band].group),
                    label: set.label.clone(),
                    members: set.members.iter().map(|&ix| self.sessions[ix].id).collect(),
                    alert: loudest(set),
                })
                .collect(),
            current: current.map(|ix| ix as u32),
            others_alert,
        }
    }
}
