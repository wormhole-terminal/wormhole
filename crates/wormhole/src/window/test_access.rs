//! What the features' tests may read and set of a window. The fields of [`Workspace`] belong to the window
//! (`pub(in crate::window)`); a feature's test drives its tab through the window the way the person does, and
//! looks at the few things below, so no field has to be wider than the window for a test's sake.

use wormhole_core::config::TabId;
use wormhole_core::view::layout::Panes;

use super::Workspace;
use super::feature::FeatureDef;
use super::focus::FocusAreas;
use super::session::Session;

impl Workspace {
    /// The window's terminals, in the sidebar's order.
    pub(crate) fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The index of the selected terminal.
    pub(crate) fn active_index(&self) -> usize {
        self.active
    }

    /// The panel's tab on show.
    pub(crate) fn right_tab(&self) -> TabId {
        self.right_tab
    }

    /// The side panels as the person left them.
    pub(crate) fn panes(&self) -> Panes {
        self.panes
    }

    /// The tick the window is at, which a test sets to meet a feature's "every n ticks".
    pub(crate) fn set_ticks(&mut self, ticks: u64) {
        self.ticks = ticks;
    }

    /// F6's areas: where the keyboard is besides the terminal.
    pub(crate) fn areas_mut(&mut self) -> &mut FocusAreas {
        &mut self.areas
    }

    /// F6's areas, read only.
    pub(crate) fn areas(&self) -> &FocusAreas {
        &self.areas
    }

    /// A "close this terminal?" question is on screen.
    pub(crate) fn closing_session_prompt(&self) -> bool {
        self.closing_session_prompt
    }

    /// The features' table this window was made with.
    pub(crate) fn feature_defs(&self) -> &'static [FeatureDef] {
        self.features
    }
}
