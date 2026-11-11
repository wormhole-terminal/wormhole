//! The right panel's tabs and the settings that change while the window runs: which features are on
//! (a tab comes or goes with its feature) and what ⌘1 to ⌘9 count along.

use crate::config::tab::TabId;
use crate::config::{FeatureId, FeatureMask};
use crate::view::sidebar::SidebarNumbers;

use super::event::{Event, Refusal};
use super::model::WorkspaceModel;

impl WorkspaceModel {
    /// Whether the panel offers `tab`: the feature that brings it is on.
    pub(super) fn tab_enabled(&self, tab: TabId) -> bool {
        FeatureId::of_tab(tab).is_none_or(|feature| self.cfg.features.has(feature))
    }

    /// The tabs the panel lists, in its order, with whether each is on.
    pub(super) fn tabs(&self) -> Vec<(TabId, bool)> {
        TabId::ALL.into_iter().map(|tab| (tab, self.tab_enabled(tab))).collect()
    }

    /// Asking for a tab is asking to see it: the panel opens, and the sidebar steps aside when the
    /// window has no room for both. A tab whose feature is off is refused.
    pub(super) fn show_panel_tab(&mut self, tab: TabId) {
        if !self.tab_enabled(tab) {
            return self.emit(Event::Refused(Refusal::Unchanged));
        }
        self.chrome_change(|chrome| {
            let width = chrome.width();
            chrome.panel_tab = tab;
            chrome.panes.reveal_panel(width);
        });
    }

    /// A feature was switched on or off. The open tab falls back when its feature went.
    pub(super) fn set_feature(&mut self, feature: FeatureId, on: bool) {
        let mask: FeatureMask = self.cfg.features.with(feature, on);
        if mask == self.cfg.features {
            return;
        }
        self.cfg.features = mask;
        if !self.tab_enabled(self.chrome.panel_tab) {
            self.chrome.panel_tab = TabId::FALLBACK;
        }
        // The tab list changed even when the open tab stayed.
        self.emit(Event::Chrome);
    }

    pub(super) fn set_numbers_follow_list(&mut self, follow: bool) {
        let numbers = if follow { SidebarNumbers::List } else { SidebarNumbers::Terminal };
        if self.cfg.numbers != numbers {
            self.cfg.numbers = numbers;
            self.rearranged();
            // The chrome snapshot says which way they count (`numbers_follow_list`).
            self.emit(Event::Chrome);
        }
    }
}
