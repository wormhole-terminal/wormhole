//! The right panel's lists under the keys: Next steps and Notes, Changes, History. Each
//! section's list is one ⇥ stop (its band leads it, and the band's toggle is in it); ↑ ↓ move,
//! → ← fold and open (a folder, a commit's files, a section) and ← on a child goes to its parent;
//! ↩ opens (a file's diff, keeping the keyboard in the list), Space ticks a next step or folds a
//! note. On a section's band only Space folds it (↩ does nothing there). While a diff is open it
//! follows the keys from file to file. Nothing stages, discards or checks out: git stays
//! read-only. Each tab says which of its rows the keys can be on and what pressing one does; this
//! walks them.

use gpui_kit::component::input as text_input;
use gpui_kit::*;
use wormhole_core::view::focus::Area;
use wormhole_core::view::focus::list::{Item, Move, band_folds, walk};

use crate::focus_keys::{Activate, activation, list_key};
use crate::window::Workspace;
use crate::window::feature::{WalkFns, Walkable};

/// How the tab on show is walked: through its feature's entity. `E` is the entity, borrowed from the
/// window or cloned out of it for a call that changes the window.
struct Walker<E>(&'static WalkFns, E);

impl Walker<&AnyEntity> {
    fn cloned(self) -> Walker<AnyEntity> {
        Walker(self.0, self.1.clone())
    }
}

impl Workspace {
    /// How the tab on show is walked, if its feature is in the table and the tab has a list.
    fn walker(&self) -> Option<Walker<&AnyEntity>> {
        let (at, tab) = self.tab_entry(self.right_tab)?;
        Some(Walker(tab.walk.as_ref()?, self.feature_entity(at)?))
    }

    /// The rows of the tab on show that the keys can be on.
    fn panel_walkable(&self, cx: &App) -> Vec<Walkable> {
        if let Some(Walker(walk, entity)) = self.walker() {
            return (walk.walkable)(entity, &self.facts(cx), cx);
        }
        // A tab without a list has no rows.
        Vec::new()
    }

    /// Where each of the tab's lists starts, as indices into its walkable rows: one ⇥ stop per
    /// section (a tab without bands is one list).
    pub(crate) fn panel_lists(&self, cx: &App) -> Vec<usize> {
        let rows = self.panel_walkable(cx);
        let mut starts: Vec<usize> =
            rows.iter().enumerate().filter(|(_, w)| w.band).map(|(ix, _)| ix).collect();
        if starts.first() != Some(&0) && !rows.is_empty() {
            starts.insert(0, 0);
        }
        starts
    }

    /// The cursor's place among the walkable rows, if it is on one of this tab's.
    fn panel_cursor(&self, rows: &[Walkable]) -> Option<usize> {
        let (tab, row) = self.areas.panel_at?;
        if tab != self.right_tab {
            return None;
        }
        // The rows may have changed under it (a section folded): the nearest one at or above.
        rows.iter().rposition(|w| w.row <= row).or((!rows.is_empty()).then_some(0))
    }

    /// Which of the tab's lists the cursor is in.
    pub(super) fn panel_list_at(&self, cx: &App) -> usize {
        let rows = self.panel_walkable(cx);
        let Some(at) = self.panel_cursor(&rows) else { return 0 };
        self.panel_lists(cx).iter().rposition(|&start| start <= at).unwrap_or(0)
    }

    /// Coming into the panel's `index`th list (`usize::MAX`: the last): the cursor on its first
    /// row, unless it is already in that list.
    pub(super) fn enter_panel_list(&mut self, index: usize, cx: &App) {
        let rows = self.panel_walkable(cx);
        let lists = self.panel_lists(cx);
        let Some(&start) = lists.get(index.min(lists.len().saturating_sub(1))) else {
            self.areas.panel_at = None;
            return;
        };
        if self.panel_cursor(&rows).is_some() && self.panel_list_at(cx) == index.min(lists.len() - 1) {
            return;
        }
        self.areas.panel_at = Some((self.right_tab, rows[start].row));
        self.panel_reveal(rows[start].row, cx);
    }

    pub(crate) fn panel_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let rows = self.panel_walkable(cx);
        let at = self.panel_cursor(&rows);
        if let Some(key) = list_key(keystroke) {
            let items: Vec<Item> = rows.iter().map(|w| w.item).collect();
            match walk(&items, at, key) {
                Move::To(ix) => {
                    let row = rows[ix].row;
                    self.areas.panel_at = Some((self.right_tab, row));
                    self.panel_reveal(row, cx);
                    // A diff that is open follows the keys, file to file.
                    if self.covered(cx) {
                        self.panel_press(row, Activate::Return, true, window, cx);
                    }
                }
                Move::Open(ix) | Move::Fold(ix) => self.panel_fold(rows[ix].row, cx),
                Move::Stay => {}
            }
            return true;
        }
        let Some(press) = activation(keystroke) else { return false };
        match at.map(|at| rows[at]) {
            Some(Walkable { row, band: true, .. }) => {
                if band_folds(press) {
                    self.panel_fold(row, cx);
                }
            }
            Some(Walkable { row, .. }) => self.panel_press(row, press, false, window, cx),
            None => {}
        }
        true
    }

    /// Press row `row` of the tab on show; `following` when the keys only moved onto it with a
    /// diff open (only a file's diff opens then). The list keeps the keyboard unless the press
    /// went into an editor (a next step or a note to write).
    fn panel_press(
        &mut self,
        row: usize,
        press: Activate,
        following: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.active_session().id;
        let keeps = match self.walker().map(Walker::cloned) {
            Some(Walker(walk, entity)) => {
                let facts = self.facts(cx);
                (walk.press)(&entity, &facts, session, row, press, following, window, cx)
            }
            None => true,
        };
        if keeps {
            self.refocus(Area::PanelContent, window, cx);
        }
    }

    /// ⌘C in the panel's list (Edit → Copy too): the row the keys are on, if its tab can copy it.
    /// `false` when it cannot, so the key goes where it would have.
    pub(crate) fn panel_copy(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        let rows = self.panel_walkable(cx);
        let Some(row) = self.panel_cursor(&rows).map(|at| rows[at].row) else { return false };
        let session = self.active_session().id;
        let Some(Walker(walk, entity)) = self.walker().map(Walker::cloned) else { return false };
        let Some(copy) = walk.copy else { return false };
        let facts = self.facts(cx);
        copy(&entity, &facts, session, row, window, cx)
    }

    /// Fold or open the row at `row` (a section, a folder, a commit's files, the done steps).
    fn panel_fold(&mut self, row: usize, cx: &mut Context<Self>) {
        let session = self.active_session().id;
        if let Some(Walker(walk, entity)) = self.walker().map(Walker::cloned) {
            let facts = self.facts(cx);
            (walk.fold)(&entity, &facts, session, row, cx);
        }
    }

    /// Scroll the tab's list to show row `row`.
    fn panel_reveal(&self, row: usize, cx: &App) {
        if let Some(Walker(walk, entity)) = self.walker() {
            (walk.reveal)(entity, &self.facts(cx), row, cx);
        }
    }

    /// The panel's list area, holding the keyboard for it: a list VoiceOver names after
    /// the tab, whose active descendant is the row the keys are on.
    pub(in crate::window) fn panel_content(&self, content: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        // The tab's own label; a window with no table has no tab, so no words either.
        let name = self.tab_entry(self.right_tab).map_or("", |(_, tab)| tab.label);
        let list = div()
            .id("panel-content")
            .role(Role::List)
            .aria_label(name)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            // The Edit menu's Copy, and ⌘C with it, when the keys are on a row that can be copied.
            .on_action(cx.listener(|this, _: &text_input::Copy, window, cx| {
                if !this.panel_copy(window, cx) {
                    cx.propagate();
                }
            }))
            .child(content);
        self.area_container(Area::PanelContent, list, cx).into_any_element()
    }
}
