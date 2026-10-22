//! Keyboard focus: F6 / ⇧F6 from area to area, the terminal included (it keeps ⇥ for its
//! program); ⇥ / ⇧⇥ from stop to stop; arrow keys inside an area, which keeps one focused item
//! (roving focus: its cursor); Esc back to the terminal, one layer at a time; and the ring that
//! shows where the keys go. The order, the skipping of hidden areas, the Esc ladder and how a list
//! is walked are core's (`wormhole_core::view::focus`).
//!
//! An area's focus handle sits on its container (the terminal list, the tab strip, the panel's
//! tabs and list, the status bar), so the item the keys are on is the container's active
//! descendant and VoiceOver follows it. A click there leaves the keyboard where it was: GPUI
//! focuses a tracked element on every mouse down in it, which the container holds off. The title
//! row's buttons are spread over two title rows, so its handle sits on an empty element at the
//! window's root.
mod bars;
mod menu;
mod panel;
#[cfg(test)]
mod window_tests;

use gpui_kit::*;
use wormhole_core::view::focus::list::{Key, TypeAhead};
use wormhole_core::view::focus::{self, Area, EscState, Escape, Stop};

use crate::workspace::{RightTab, Workspace, actions};

pub(super) use bars::TitleButton;
pub(in crate::workspace) use menu::key_menu_origin;
pub(in crate::workspace) use panel::{Activate, Walkable};

actions!(focus_area, [NextStop, PreviousStop]);

/// The key context of an area's container: where ⇥ / ⇧⇥ step from stop to stop.
const AREA_CONTEXT: &str = "FocusArea";

/// Bind ⇥ and ⇧⇥ inside the areas. They are the conventions for moving through a window, like
/// a text field's own keys, not shortcuts, so they aren't in core's key table. The terminal takes
/// its keys before any binding, so it keeps them; GPUI Kit's window root binds them to step through
/// its controls, and this deeper context wins over it.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextStop, Some(AREA_CONTEXT)),
        KeyBinding::new("shift-tab", PreviousStop, Some(AREA_CONTEXT)),
    ]);
}

/// A sidebar item the keys can be on: one of the person's groups (by its id) or a terminal (by its
/// id).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SidebarStop {
    Group(u32),
    Row(i64),
}

pub(super) struct FocusAreas {
    title_row: FocusHandle,
    sidebar: FocusHandle,
    tabs: FocusHandle,
    panel_tabs: FocusHandle,
    panel_content: FocusHandle,
    status_bar: FocusHandle,
    /// The title row's button the keys are on.
    pub(super) title: usize,
    pub(super) sidebar_at: Option<SidebarStop>,
    /// The panel row the keys are on: its tab and its place among that tab's rows.
    pub(super) panel_at: Option<(RightTab, usize)>,
    /// The status bar's item the keys are on.
    pub(super) status: usize,
    /// Letters typed in the sidebar to jump to a name.
    pub(super) typed: TypeAhead,
}

impl FocusAreas {
    pub fn new(cx: &mut App) -> Self {
        Self {
            title_row: cx.focus_handle(),
            sidebar: cx.focus_handle(),
            tabs: cx.focus_handle(),
            panel_tabs: cx.focus_handle(),
            panel_content: cx.focus_handle(),
            status_bar: cx.focus_handle(),
            title: 0,
            sidebar_at: None,
            panel_at: None,
            status: 0,
            typed: TypeAhead::default(),
        }
    }

    /// The handle an area other than the terminal holds the focus with.
    fn handle(&self, area: Area) -> Option<&FocusHandle> {
        Some(match area {
            Area::TitleRow => &self.title_row,
            Area::Sidebar => &self.sidebar,
            Area::Tabs => &self.tabs,
            Area::Terminal => return None,
            Area::PanelTabs => &self.panel_tabs,
            Area::PanelContent => &self.panel_content,
            Area::StatusBar => &self.status_bar,
        })
    }

    /// The area one of these handles holds the focus for.
    fn holder(&self, window: &Window) -> Option<Area> {
        Area::ALL.into_iter().find(|&area| self.handle(area).is_some_and(|h| h.is_focused(window)))
    }
}

/// Whether `window` is the one in front, as far as the ring goes. devctl can say so in place of
/// macOS (`window-front`), to see the ring without taking the front from the app the person uses.
pub(in crate::workspace) fn in_front(window: &Window) -> bool {
    #[cfg(feature = "devctl")]
    if let Some(front) = FRONT.get() {
        return front;
    }
    window.is_window_active()
}

#[cfg(feature = "devctl")]
thread_local! {
    static FRONT: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// devctl's `window-front on|off|system`.
#[cfg(feature = "devctl")]
pub(crate) fn set_front(front: Option<bool>) {
    FRONT.set(front);
}

/// The arrow keys a list walks with (⌥↑ / ⌥↓: the first and the last item).
pub(in crate::workspace) fn list_key(keystroke: &Keystroke) -> Option<Key> {
    let m = &keystroke.modifiers;
    if m.control || m.platform || m.shift || m.function && !is_arrow(&keystroke.key) {
        return None;
    }
    Some(match (keystroke.key.as_str(), m.alt) {
        ("up", false) => Key::Up,
        ("down", false) => Key::Down,
        ("left", false) => Key::Left,
        ("right", false) => Key::Right,
        ("up", true) => Key::First,
        ("down", true) => Key::Last,
        _ => return None,
    })
}

/// macOS reports the arrows with the function modifier set.
fn is_arrow(key: &str) -> bool {
    matches!(key, "up" | "down" | "left" | "right")
}

/// Return and Space, with no modifier: what presses the item the keys are on.
pub(in crate::workspace) fn activation(keystroke: &Keystroke) -> Option<Activate> {
    if keystroke.modifiers.modified() {
        return None;
    }
    match keystroke.key.as_str() {
        "enter" => Some(Activate::Return),
        "space" => Some(Activate::Space),
        _ => None,
    }
}

impl Workspace {
    /// The area that has the keyboard, if any: the terminal, a note being written (the panel's
    /// content), or an area the keys moved to.
    pub(crate) fn focus_area(&self, window: &Window, cx: &App) -> Option<Area> {
        if self.active_terminal().focus_handle(cx).is_focused(window) {
            return Some(Area::Terminal);
        }
        if self.editing_note.is_some() {
            return Some(Area::PanelContent);
        }
        self.areas.holder(window)
    }

    /// Whether an area is on screen to move to.
    fn area_shown(&self, area: Area) -> bool {
        match area {
            Area::TitleRow | Area::Terminal => true,
            Area::Sidebar => self.sidebar_shown(),
            // A diff covers the terminal and its title row's tabs.
            Area::Tabs => self.diff_view.is_none(),
            Area::PanelTabs | Area::PanelContent => self.notes_visible(),
            Area::StatusBar => self.status_bar_drawn.is_some(),
        }
    }

    /// Whether `area`'s ring shows: it has the keyboard, the last input was a key (a click moves
    /// the focus without a ring), and the window is in front (the ring hides in an inactive
    /// window; selection stays).
    pub(super) fn ring(&self, area: Area, window: &Window) -> bool {
        self.areas.handle(area).is_some_and(|h| h.is_focused(window))
            && window.last_input_was_keyboard()
            && in_front(window)
    }

    /// F6 (`1`) / ⇧F6 (`-1`): the next or previous area on screen gets the keyboard.
    pub(super) fn step_focus_area(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let to = focus::step(self.focus_area(window, cx), delta, |area| self.area_shown(area));
        // Back into the title row from behind (⇧F6), the last button is the nearest.
        let index = if to == Area::TitleRow && delta < 0 { usize::MAX } else { 0 };
        self.focus_stop(Stop { area: to, index }, window, cx);
    }

    /// How many ⇥ stops an area has now: none while it is hidden; the title row's buttons
    /// only with macOS's Keyboard navigation on; one per list in the panel.
    fn stops(&self, area: Area, cx: &App) -> usize {
        if !self.area_shown(area) {
            return 0;
        }
        match area {
            Area::TitleRow if crate::platform::a11y::keyboard_navigation() => self.title_buttons(cx).len(),
            Area::TitleRow => 0,
            Area::PanelContent => self.panel_lists(cx).len().max(1),
            _ => 1,
        }
    }

    /// The stop the keys are on now, for ⇥: in the title row its button, in the panel its list.
    fn current_stop(&self, window: &Window, cx: &App) -> Option<Stop> {
        let area = self.focus_area(window, cx)?;
        let index = match area {
            Area::TitleRow => self.areas.title,
            Area::PanelContent => self.panel_list_at(cx),
            _ => 0,
        };
        Some(Stop { area, index })
    }

    /// ⇥ (`1`) / ⇧⇥ (`-1`): the next or previous stop.
    fn step_stop(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let to = focus::tab_step(self.current_stop(window, cx), delta, |area| self.stops(area, cx));
        self.focus_stop(to, window, cx);
    }

    /// Give `stop` the keyboard, putting its area's cursor on it (or, coming into an area with no
    /// cursor yet, on the selected terminal, the first list, the first button).
    fn focus_stop(&mut self, stop: Stop, window: &mut Window, cx: &mut Context<Self>) {
        match stop.area {
            Area::Terminal => {
                self.focus_terminal(window, cx);
                cx.notify();
                return;
            }
            Area::TitleRow => {
                let last = self.title_buttons(cx).len().saturating_sub(1);
                self.areas.title = stop.index.min(last);
            }
            Area::Sidebar => self.enter_sidebar(cx),
            Area::PanelContent => self.enter_panel_list(stop.index, cx),
            Area::StatusBar => {
                self.areas.status = self.areas.status.min(self.status_items().len().saturating_sub(1));
            }
            Area::Tabs | Area::PanelTabs => {}
        }
        if let Some(handle) = self.areas.handle(stop.area) {
            window.focus(handle, cx);
        }
        cx.notify();
    }

    /// Esc while the terminal has the keys, which a focused terminal would otherwise send its
    /// program before any listener hears it: the ladder's rungs above the shell (a diff covering
    /// the terminal) take it.
    pub(in crate::workspace) fn terminal_escape(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.active_terminal().focus_handle(cx).is_focused(window) && self.escape_key(window, cx)
    }

    /// Esc, one layer at a time: a panel drag first, then the ladder (a rename field, the
    /// diff, back to the terminal; in the terminal it is the program's). Overlays (the palette,
    /// the sheet, a dialog) and text fields answer Esc themselves.
    pub(super) fn escape_key(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.cancel_resize(cx) || self.cancel_pane_drag(cx) || self.close_path_card(cx) {
            return true;
        }
        if self.overlay.is_some() || self.first_run.is_open() {
            return false;
        }
        let in_area = self.areas.holder(window).is_some();
        let state = EscState {
            terminal: self.active_terminal().focus_handle(cx).is_focused(window),
            // A native menu is modal: no key reaches the workspace while one is open.
            popup: false,
            detail: self.diff_view.is_some() || self.active_terminal().read(cx).find_open(),
        };
        match focus::escape(state) {
            // A row's menu cancels itself; the terminal hands Esc to its program.
            Escape::ToShell | Escape::ClosePopup => false,
            // The find bar closes and the keyboard goes back to its terminal.
            Escape::CloseDetail if self.diff_view.is_none() => {
                let terminal = self.active_terminal().clone();
                terminal.update(cx, |terminal, cx| terminal.close_find(window, cx));
                true
            }
            Escape::CloseDetail => {
                self.close_diff(window, cx);
                // Its list keeps the keyboard (a second Esc goes to the terminal).
                if in_area {
                    self.refocus(Area::PanelContent, window, cx);
                }
                true
            }
            // Only from an area, or from nowhere: a text field's Esc is its own.
            Escape::ToTerminal if in_area || window.focused(cx).is_none() => {
                self.focus_terminal(window, cx);
                cx.notify();
                true
            }
            Escape::ToTerminal => false,
        }
    }

    /// Give the keyboard back to `area` after an action that moved it (opening a terminal while
    /// browsing the list, a tab while browsing the tabs).
    pub(in crate::workspace) fn refocus(&self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handle) = self.areas.handle(area) {
            window.focus(handle, cx);
        }
    }

    /// Make `element` the container that holds `area`'s keyboard focus: the keys (arrows, ↩,
    /// Space, letters, ⇥) go to it while it, not something inside it, has the focus.
    pub(super) fn area_container(
        &self,
        area: Area,
        element: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(handle) = self.areas.handle(area) else { return element };
        element
            .track_focus(handle)
            .key_context(AREA_CONTEXT)
            // A click in an area leaves the keyboard where it was; keys bring it here.
            .on_any_mouse_down(|_, window, _| window.prevent_default())
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                this.area_key(area, event, window, cx)
            }))
            .on_action(cx.listener(|this, _: &NextStop, window, cx| this.stop_key(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousStop, window, cx| this.stop_key(-1, window, cx)))
    }

    /// ⇥ in an area: the next stop, unless a text field inside it has the keys (then the field,
    /// or GPUI Kit's own stepping through controls, takes it).
    fn stop_key(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.areas.holder(window).is_none() {
            cx.propagate();
            return;
        }
        self.step_stop(delta, window, cx);
    }

    /// A key in `area` while its container itself has the keyboard.
    fn area_key(&mut self, area: Area, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.areas.handle(area).is_some_and(|h| h.is_focused(window)) {
            return;
        }
        // ⌃↩ in a list: its row's menu.
        if matches!(area, Area::Sidebar | Area::Tabs | Area::PanelContent)
            && menu::is_menu_key(&event.keystroke)
        {
            self.row_menu_key(area, window, cx);
            cx.stop_propagation();
            return;
        }
        let handled = match area {
            Area::TitleRow => self.title_key(&event.keystroke, window, cx),
            Area::Sidebar => self.sidebar_key(&event.keystroke, window, cx),
            Area::Tabs => self.tabs_key(&event.keystroke, window, cx),
            Area::PanelTabs => self.panel_tabs_key(&event.keystroke, window, cx),
            Area::PanelContent => self.panel_key(&event.keystroke, window, cx),
            Area::StatusBar => self.status_key(&event.keystroke, window, cx),
            Area::Terminal => false,
        };
        if handled {
            cx.stop_propagation();
            cx.notify();
        }
    }

    /// The empty element that holds the title row's focus, at the window's root (its buttons are
    /// in two title rows).
    pub(super) fn render_focus_holders(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.area_container(Area::TitleRow, div().id("title-row-focus").absolute().size_0(), cx)
    }

    /// Where the keys are, for `devctl state`: the area, the item its cursor is on, and whether
    /// the ring shows.
    #[cfg(feature = "devctl")]
    pub(crate) fn focus_detail(&self, window: &Window, cx: &App) -> String {
        let Some(area) = self.focus_area(window, cx) else { return "none".into() };
        let at = match area {
            Area::TitleRow => self.title_buttons(cx).get(self.areas.title).map(|b| format!("{b:?}")),
            Area::Sidebar => self.areas.sidebar_at.as_ref().map(|at| match at {
                SidebarStop::Group(id) => format!("group {id}"),
                SidebarStop::Row(id) => format!("terminal id={id}"),
            }),
            Area::PanelContent => {
                self.areas.panel_at.map(|(tab, row)| format!("{} row {row}", tab_name(tab)))
            }
            Area::StatusBar => self.status_items().get(self.areas.status).map(|item| format!("{item:?}")),
            Area::Tabs | Area::PanelTabs | Area::Terminal => None,
        };
        let ring = if self.ring(area, window) { " ring" } else { "" };
        match at {
            Some(at) => format!("{} {at}{ring}", area.name()),
            None => format!("{}{ring}", area.name()),
        }
    }
}

#[cfg(feature = "devctl")]
fn tab_name(tab: RightTab) -> &'static str {
    match tab {
        RightTab::Notes => "notes",
        RightTab::Changes => "changes",
        RightTab::History => "history",
        RightTab::Project => "project",
        RightTab::Usage => "usage",
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::Keystroke;
    use wormhole_core::view::focus::list::Key;

    use super::{Activate, activation, list_key};

    fn key(text: &str) -> Keystroke {
        Keystroke::parse(text).expect("a keystroke")
    }

    #[test]
    fn the_arrows_walk_and_option_jumps_to_the_ends() {
        assert_eq!(list_key(&key("up")), Some(Key::Up));
        assert_eq!(list_key(&key("right")), Some(Key::Right));
        assert_eq!(list_key(&key("alt-up")), Some(Key::First));
        assert_eq!(list_key(&key("alt-down")), Some(Key::Last));
        assert_eq!(list_key(&key("cmd-up")), None, "⌘↑ is an app key");
        assert_eq!(list_key(&key("shift-down")), None);
        assert_eq!(list_key(&key("a")), None);
    }

    #[test]
    fn return_and_space_press_without_modifiers() {
        assert_eq!(activation(&key("enter")), Some(Activate::Return));
        assert_eq!(activation(&key("space")), Some(Activate::Space));
        assert_eq!(activation(&key("cmd-enter")), None);
        assert_eq!(activation(&key("tab")), None);
    }
}
