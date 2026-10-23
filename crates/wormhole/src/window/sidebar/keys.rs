//! The terminal list under the keys: bands and terminals are both stops, the rows of a
//! folded band are skipped; ↑ ↓ move, ← goes from a terminal to its band and folds an open band,
//! → opens a folded band and goes from an open one to its first terminal, ⌥↑ ⌥↓ go to the ends,
//! letters jump to the next name that starts with them. ↩ opens the terminal and gives it the
//! keyboard; Space shows it and keeps the keyboard in the list (browsing). On a band Space folds
//! or opens it and ↩ does nothing. The list has no key of its own for Close:
//! ⌘W closes the selected terminal (the one shown, which is not always the one the keys are on),
//! and the row's menu (⌃↩) closes the one it was opened on.
//!
//! The list is the window's keyboard area for the sidebar: it holds the focus handle, hears the keys
//! that reach it and asks the window by an event for the one thing that is the window's, stepping on
//! to the next area (⇥).

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::view::focus::list::{Item, Move, band_folds, type_ahead, walk};
use wormhole_core::view::sidebar::Entry;

use super::{Sidebar, SidebarEvent};
use crate::focus_keys::{
    AREA_CONTEXT, Activate, NextStop, PreviousStop, activation, in_front, is_menu_key, list_key,
};

/// A sidebar item the keys can be on: one of the person's groups (by its id) or a terminal (by its
/// id).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) enum SidebarStop {
    Group(u32),
    Row(i64),
}

/// The list as the keys walk it, top to bottom.
struct Walk {
    stops: Vec<SidebarStop>,
    items: Vec<Item>,
    names: Vec<String>,
    /// The terminal each terminal's stop opens (`None` for a band).
    terminals: Vec<Option<i64>>,
}

impl Sidebar {
    fn walk(&self, cx: &App) -> Walk {
        let infos = &self.input.rows;
        let listing = self.listing(cx);
        let mut list =
            Walk { stops: Vec::new(), items: Vec::new(), names: Vec::new(), terminals: Vec::new() };
        for entry in &listing.entries {
            match *entry {
                Entry::Band { band, .. } => {
                    let band = &listing.bands[band];
                    list.stops.push(SidebarStop::Group(band.group));
                    list.items.push(Item { level: 0, fold: Some(!band.folded) });
                    list.names.push(band.label.clone());
                    list.terminals.push(None);
                }
                Entry::Row { session, band, .. } => {
                    let info = &infos[session];
                    list.stops.push(SidebarStop::Row(info.id));
                    list.items.push(Item { level: u8::from(band.is_some()), fold: None });
                    list.names.push(info.name.to_string());
                    list.terminals.push(Some(info.id));
                }
            }
        }
        list
    }

    /// Coming into the list (F6, ⇥): the keys go back where they were, or onto the selected
    /// terminal (its band while it is folded away), or the first item.
    pub(in crate::window) fn enter(&mut self, cx: &App) {
        let list = self.walk(cx);
        if self.keys_at.as_ref().is_some_and(|at| list.stops.contains(at)) {
            return;
        }
        let selected = self.input.active_id().map(SidebarStop::Row);
        // Folded away, the selected terminal is its group's head.
        let group =
            self.input.active_id().and_then(|id| self.view.groups.group_of(id)).map(SidebarStop::Group);
        self.keys_at = [selected, group]
            .into_iter()
            .flatten()
            .find(|stop| list.stops.contains(stop))
            .or_else(|| list.stops.first().cloned());
    }

    /// Whether the keys are on `stop` (it is the list's active descendant), and whether its ring
    /// shows: the list has the keyboard, the last input was a key (a click moves the focus without a
    /// ring), and the window is in front.
    pub(super) fn keyed(&self, stop: &SidebarStop, window: &Window) -> (bool, bool) {
        let keyed = self.keys_at.as_ref() == Some(stop);
        (keyed, keyed && self.ring(window))
    }

    /// Whether the list's ring shows.
    pub(super) fn ring(&self, window: &Window) -> bool {
        self.focus.is_focused(window) && window.last_input_was_keyboard() && in_front(window)
    }

    /// Make `list` the container that holds the sidebar's keyboard focus: the keys (arrows, ↩,
    /// Space, letters, ⇥) go to it while it, not something inside it, has the focus.
    pub(super) fn area_container(&self, list: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        list.track_focus(&self.focus)
            .key_context(AREA_CONTEXT)
            // A click in an area leaves the keyboard where it was; keys bring it here.
            .on_any_mouse_down(|_, window, _| window.prevent_default())
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, window, cx| this.on_area_key(event, window, cx)),
            )
            .on_action(cx.listener(|this, _: &NextStop, window, cx| this.stop_key(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousStop, window, cx| this.stop_key(-1, window, cx)))
    }

    /// ⇥ in the list: the next stop of the window, unless a text field inside it has the keys (then
    /// the field, or GPUI Kit's own stepping through controls, takes it).
    fn stop_key(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            cx.propagate();
            return;
        }
        cx.emit(SidebarEvent::StepStop(delta));
    }

    /// A key while the list's container itself has the keyboard.
    fn on_area_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return;
        }
        // ⌃↩ in the list: its row's menu.
        if is_menu_key(&event.keystroke) {
            self.open_key_menu(window, cx);
            cx.stop_propagation();
            return;
        }
        if self.key(&event.keystroke, cx) {
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let list = self.walk(cx);
        let at = self.keys_at.as_ref().and_then(|at| list.stops.iter().position(|s| s == at));
        if let Some(key) = list_key(keystroke) {
            match walk(&list.items, at, key) {
                Move::To(ix) => self.move_to(&list, ix),
                Move::Open(ix) | Move::Fold(ix) => self.fold_at(&list, ix, cx),
                Move::Stay => {}
            }
            return true;
        }
        if let Some(press) = activation(keystroke) {
            let Some(at) = at else { return true };
            match list.terminals[at] {
                // Space shows it while the keys keep browsing.
                Some(id) => cx.emit(SidebarEvent::Activate { id, then_focus_list: press == Activate::Space }),
                None if band_folds(press) => self.fold_at(&list, at, cx),
                None => {}
            }
            return true;
        }
        self.type_ahead(&list, at, keystroke)
    }

    fn move_to(&mut self, list: &Walk, ix: usize) {
        self.keys_at = list.stops.get(ix).cloned();
        // The list's children are its entries, in order.
        self.scroll.scroll_to_item(ix);
    }

    fn fold_at(&mut self, list: &Walk, ix: usize, cx: &mut Context<Self>) {
        if let Some(&SidebarStop::Group(group)) = list.stops.get(ix) {
            self.toggle_group_band(group, cx);
        }
    }

    /// A letter or digit typed in the list: the next name that starts with what was typed in the
    /// last second.
    fn type_ahead(&mut self, list: &Walk, at: Option<usize>, keystroke: &Keystroke) -> bool {
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
        let typed = self.typed.push(ch, Instant::now()).to_string();
        let names: Vec<&str> = list.names.iter().map(String::as_str).collect();
        if let Some(ix) = type_ahead(&names, at, &typed) {
            self.move_to(list, ix);
        }
        true
    }
}
