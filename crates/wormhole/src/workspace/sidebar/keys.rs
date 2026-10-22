//! The terminal list under the keys: bands and terminals are both stops, the rows of a
//! folded band are skipped; ↑ ↓ move, ← goes from a terminal to its band and folds an open band,
//! → opens a folded band and goes from an open one to its first terminal, ⌥↑ ⌥↓ go to the ends,
//! letters jump to the next name that starts with them. ↩ opens the terminal and gives it the
//! keyboard; Space shows it and keeps the keyboard in the list (browsing). On a band Space folds
//! or opens it and ↩ does nothing. The list has no key of its own for Close:
//! ⌘W closes the selected terminal (the one shown, which is not always the one the keys are on),
//! and the row's menu (⌃↩) closes the one it was opened on.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::view::focus::list::{Item, Move, band_folds, type_ahead, walk};
use wormhole_core::view::sidebar::Entry;

use crate::workspace::Workspace;
use crate::workspace::focus::{Activate, SidebarStop, activation, list_key};

/// The list as the keys walk it, top to bottom.
struct Walk {
    stops: Vec<SidebarStop>,
    items: Vec<Item>,
    names: Vec<String>,
    /// The session each terminal's stop opens (`None` for a band).
    sessions: Vec<Option<usize>>,
}

impl Workspace {
    fn sidebar_walk(&self, cx: &App) -> Walk {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let mut list = Walk { stops: Vec::new(), items: Vec::new(), names: Vec::new(), sessions: Vec::new() };
        for entry in &listing.entries {
            match *entry {
                Entry::Band { band, .. } => {
                    let band = &listing.bands[band];
                    list.stops.push(SidebarStop::Group(band.group));
                    list.items.push(Item { level: 0, fold: Some(!band.folded) });
                    list.names.push(band.label.clone());
                    list.sessions.push(None);
                }
                Entry::Row { session, band, .. } => {
                    let info = &infos[session];
                    list.stops.push(SidebarStop::Row(info.id));
                    list.items.push(Item { level: u8::from(band.is_some()), fold: None });
                    list.names.push(info.name.to_string());
                    list.sessions.push(Some(info.ix));
                }
            }
        }
        list
    }

    /// Coming into the list (F6, ⇥): the keys go back where they were, or onto the selected
    /// terminal (its band while it is folded away), or the first item.
    pub(in crate::workspace) fn enter_sidebar(&mut self, cx: &App) {
        let list = self.sidebar_walk(cx);
        if self.areas.sidebar_at.as_ref().is_some_and(|at| list.stops.contains(at)) {
            return;
        }
        let selected = self.sessions.get(self.active).map(|s| SidebarStop::Row(s.id));
        // Folded away, the selected terminal is its group's head.
        let group = self.active_group().map(SidebarStop::Group);
        self.areas.sidebar_at = [selected, group]
            .into_iter()
            .flatten()
            .find(|stop| list.stops.contains(stop))
            .or_else(|| list.stops.first().cloned());
    }

    /// Whether the keys are on `stop` (it is the list's active descendant), and whether its ring
    /// shows.
    pub(super) fn sidebar_keyed(&self, stop: &SidebarStop, window: &Window) -> (bool, bool) {
        let keyed = self.areas.sidebar_at.as_ref() == Some(stop);
        (keyed, keyed && self.ring(wormhole_core::view::focus::Area::Sidebar, window))
    }

    pub(in crate::workspace) fn sidebar_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let list = self.sidebar_walk(cx);
        let at = self.areas.sidebar_at.as_ref().and_then(|at| list.stops.iter().position(|s| s == at));
        if let Some(key) = list_key(keystroke) {
            match walk(&list.items, at, key) {
                Move::To(ix) => self.sidebar_move(&list, ix),
                Move::Open(ix) | Move::Fold(ix) => self.sidebar_fold(&list, ix, cx),
                Move::Stay => {}
            }
            return true;
        }
        if let Some(press) = activation(keystroke) {
            let Some(at) = at else { return true };
            match list.sessions[at] {
                Some(ix) => {
                    self.activate(ix, window, cx);
                    // Space shows it while the keys keep browsing.
                    if press == Activate::Space {
                        self.refocus(wormhole_core::view::focus::Area::Sidebar, window, cx);
                    }
                }
                None if band_folds(press) => self.sidebar_fold(&list, at, cx),
                None => {}
            }
            return true;
        }
        self.sidebar_type_ahead(&list, at, keystroke)
    }

    fn sidebar_move(&mut self, list: &Walk, ix: usize) {
        self.areas.sidebar_at = list.stops.get(ix).cloned();
        // The list's children are its entries, in order.
        self.sidebar.scroll.scroll_to_item(ix);
    }

    fn sidebar_fold(&mut self, list: &Walk, ix: usize, cx: &mut Context<Self>) {
        if let Some(&SidebarStop::Group(group)) = list.stops.get(ix) {
            self.toggle_group_band(group, cx);
        }
    }

    /// A letter or digit typed in the list: the next name that starts with what was typed in the
    /// last second.
    fn sidebar_type_ahead(&mut self, list: &Walk, at: Option<usize>, keystroke: &Keystroke) -> bool {
        let m = &keystroke.modifiers;
        if m.platform || m.control || m.function {
            return false;
        }
        let Some(ch) = keystroke.key_char.as_deref().and_then(|text| {
            let mut chars = text.chars();
            chars.next().filter(|c| !c.is_control() && !c.is_whitespace() && chars.next().is_none())
        }) else {
            return false;
        };
        let typed = self.areas.typed.push(ch, Instant::now()).to_string();
        let names: Vec<&str> = list.names.iter().map(String::as_str).collect();
        if let Some(ix) = type_ahead(&names, at, &typed) {
            self.sidebar_move(list, ix);
        }
        true
    }
}
