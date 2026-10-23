//! Changes: what the terminal's repository has changed, as a list or a folder tree, the diff of a file
//! that opens over the terminal, and the review comments written on diffs. Always on.
//!
//! One [`Changes`] entity per window holds, for each terminal of the window, the list's folds and
//! rows and the comments ([`ChangesOf`]), and for the window the diff that is open (it belongs to the
//! terminal it was opened from) and the "compare with" picker. The window talks to it through the
//! table (`def`): its tab, its keys, the diff it lays over the terminal, the files it asks the window's
//! repository read to watch; it talks to the window by [`FeatureEvent`]s and never holds the
//! `Workspace`. The status itself is the window's (`window::repo`): the sidebar, the title row and
//! the status bar show it too, and this feature is lent it through the facts.

use std::collections::HashMap;

use gpui_kit::*;
use wormhole_core::git::review::Review;

use super::diff::{Commenting, DiffView};
pub(super) use super::picker::PickerHost;
use super::picker::{BranchPicker, Picker};
use crate::window::facts::WindowFacts;
use crate::window::feature::FeatureEvent;
use crate::window::feature::RepoRequest;
use wormhole_core::storage::paths::ProfileId;

mod def;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod list;
mod rows;
#[cfg(test)]
mod tests;
mod view;

pub(in crate::window) use def::DEF;
use list::ChangesList;
use rows::Group;
pub(super) use rows::display_order;

/// The window's Changes: every terminal's list and comments, the diff on view and the picker.
pub(in crate::window) struct Changes {
    pub(super) of: HashMap<i64, ChangesOf>,
    /// The diff on view, covering the terminal it was opened from (selecting another closes it).
    pub(super) diff: Option<DiffView>,
    /// The comment dialog of a diff line, while it is up.
    pub(super) commenting: Option<Commenting>,
    /// "Compare with".
    picker: BranchPicker,
    /// The window as the feature last heard it, for what is done between frames (a settings change
    /// that rebuilds the diff, a diff that lands).
    pub(super) profile: ProfileId,
    pub(super) terminal_width: f32,
}

impl EventEmitter<FeatureEvent> for Changes {}

impl PickerHost for Changes {
    fn picker(&self) -> &BranchPicker {
        &self.picker
    }

    fn picker_mut(&mut self) -> &mut BranchPicker {
        &mut self.picker
    }

    /// Reads the status against the branch and drops the diff: its file may not be in the new list.
    fn pick(&mut self, session: i64, branch: Option<String>, cx: &mut Context<Self>) {
        self.diff = None;
        cx.emit(FeatureEvent::Repo(RepoRequest::Compare { session, branch }));
        cx.notify();
    }
}

/// One terminal's Changes: the sections the person folded, the rows built from the status, the
/// folders folded in the tree and the comments written on its diffs.
#[derive(Default)]
pub(super) struct ChangesOf {
    /// Sections the person folded (`true`) or unfolded; the others follow the size rule
    /// (`wormhole_core::view::panel::section_folded`).
    folds: HashMap<Group, bool>,
    list: ChangesList,
    /// Folders folded in the tree, by [`rows::collapse_key`].
    collapsed: std::collections::HashSet<String>,
    /// The comments written on its diffs and not pasted into its terminal yet.
    pub(super) review: Review,
}

impl Changes {
    pub(in crate::window) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            of: HashMap::new(),
            diff: None,
            commenting: None,
            picker: BranchPicker::new(Picker::Compare, window, cx),
            profile: ProfileId::default_profile(),
            terminal_width: wormhole_core::view::layout::PANEL_MIN,
        }
    }

    /// Takes in what the window says of itself: whose settings it draws with and how much room the
    /// terminal's column has.
    pub(super) fn follow(&mut self, facts: &WindowFacts<'_>) {
        if self.profile != *facts.profile {
            self.profile = facts.profile.clone();
        }
        self.terminal_width = f32::from(facts.terminal_width);
    }

    pub(super) fn of_mut(&mut self, session: i64) -> &mut ChangesOf {
        self.of.entry(session).or_default()
    }

    /// What opening a file of `session`'s list needs, as the rows were last built.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn target(&self, session: i64) -> Option<std::rc::Rc<list::DiffTarget>> {
        Some(self.of.get(&session)?.list.built.as_ref()?.target.clone())
    }

    /// The comments written for `session`.
    pub(super) fn review(&self, session: i64) -> Option<&Review> {
        self.of.get(&session).map(|of| &of.review)
    }

    /// Another terminal was selected: the diff and the picker belonged to the one before.
    pub(super) fn selected(&mut self, cx: &mut Context<Self>) {
        let had_diff = self.diff.take().is_some();
        let had_picker = self.picker.is_open();
        self.picker.close();
        if had_diff || had_picker {
            cx.notify();
        }
    }

    /// A terminal is gone from this window (closed, or moved to another).
    pub(super) fn forget(&mut self, session: i64, cx: &mut Context<Self>) {
        self.of.remove(&session);
        if self.diff.as_ref().is_some_and(|diff| diff.session() == session) {
            self.diff = None;
            cx.notify();
        }
    }
}
