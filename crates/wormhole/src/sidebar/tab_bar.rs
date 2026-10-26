//! The terminals as tabs, instead of the sidebar (Settings → Show terminals in → Tabs), as RF-02 draws
//! it: one 38 pt row with the traffic lights, the profile chip (while there are two profiles or
//! more), the group switcher (`switcher`), that group's terminals as tabs and the +. The sets of tabs
//! are the list's groups and the ungrouped terminals (core `sidebar::tabs`); ⌃⌥[ ⌃⌥] and ⌃⌥1–9
//! move between them in either layout.
use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::sidebar::Urgency;
use wormhole_core::view::sidebar::hints::keycap;
use wormhole_core::view::sidebar::spoken;
use wormhole_core::view::sidebar::tabs::{
    TabSet, current_set, others_need_you, set_numbered, step_set, tab_sets,
};
use wormhole_core::view::sidebar::{Entry, Listing};

use super::facts::RowInfo;
use super::glass::{GlassKind, Insets, TAB_ROW_CLIP, clip_probe, glass_key, probe};
use super::row::{Frame, glyph_for, tile_for};
use super::{NewTerminal, Sidebar, SidebarEvent};
use crate::settings::{UiColors, store_in, ui_in};
use crate::style;
use crate::style::TerminalTab;

/// The gap between the tabs and the terminal's surface under them.
const TAB_GAP: f32 = 8.0;
/// The tab bar's height under the title row: a tab's, and the gap above the terminal's surface.
pub(crate) const TAB_ROW_H: f32 = style::TAB_H + TAB_GAP;
/// How much of the selected tab's fill lies over its glass.
const GLASS_SELECTED: f32 = 0.7;
/// The hairlines between the bar's parts.
const PART_RULE_H: f32 = 16.0;
/// A tab's name size (`text_label`), for measuring where a long one fades out.
const TAB_NAME_REM: f32 = 0.8125;
/// What the ⌘N keycap takes from the end of a tab's name while ⌘ is held, its gap included.
const HINT_W: f32 = 28.0;
/// How far the × stands in from the tab's right side: its glyph ends on the tab's 8 pt padding.
const CLOSE_RIGHT: f32 = 8.0 - (style::CONTROL_H_SM - style::GLYPH) / 2.0;

/// What the tab bar remembers between frames.
#[derive(Default)]
pub(super) struct TabBarState {
    /// The terminal last selected in each set, by its key: where opening the set goes back to.
    last: HashMap<String, i64>,
    /// The group switcher being open: its search field.
    pub(super) switcher: Option<super::switcher::Switcher>,
    /// The row of tabs, which scrolls sideways once they are at their narrowest, and the terminal it
    /// last brought into view.
    scroll: ScrollHandle,
    scrolled_to: Option<i64>,
    /// How many times the bar was built, for the tests that check a notification drew it.
    #[cfg(test)]
    built: usize,
}

/// One set as the bar draws it.
pub(super) struct SetLook {
    pub(super) key: String,
    pub(super) color: Option<Hsla>,
    /// The colour its card is washed with (`GroupPaint::wash`).
    pub(super) wash: Option<Hsla>,
    pub(super) group: Option<u32>,
    pub(super) pinned: bool,
}

/// The sets of tabs, how each looks, and the one the selected terminal is in.
pub(super) struct SetView {
    pub(super) sets: Vec<TabSet>,
    pub(super) looks: Vec<SetLook>,
    /// `None` only while the selected terminal is in no set (it is always in one).
    pub(super) current: Option<usize>,
}

impl Sidebar {
    /// The sets of the list as it is now, each with its look, and the current one; the terminal
    /// last selected in the current one is remembered, so coming back to the set lands there.
    pub(super) fn set_view(&mut self, listing: &Listing, cx: &App) -> SetView {
        let sets = tab_sets(listing);
        let current = current_set(&sets, self.input.active);
        let looks: Vec<SetLook> = sets.iter().map(|set| self.set_look(listing, set, cx)).collect();
        if let (Some(look), Some(id)) = (current.and_then(|at| looks.get(at)), self.input.active_id()) {
            self.tabs.last.insert(look.key.clone(), id);
        }
        SetView { sets, looks, current }
    }

    /// How many times the bar was built.
    #[cfg(test)]
    pub(crate) fn tab_bar_builds(&self) -> usize {
        self.tabs.built
    }

    /// The tab bar, as macOS Terminal's: none while there is only one terminal and no group to
    /// switch to. The bar shows once there is more than one set (a group beside another, or beside
    /// the ungrouped terminals), where the group switcher is, or the current set has more than one
    /// terminal. Wherever it shows, the current set's terminals are its tabs, even when it is one.
    pub(crate) fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        #[cfg(test)]
        {
            self.tabs.built += 1;
        }
        let input = self.input.clone();
        let infos = &input.rows;
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        let current = view.current.unwrap_or(0);
        let switch = view.sets.len() > 1;
        let several = view.sets.get(current).is_some_and(|set| set.members.len() > 1);
        if !switch && !several {
            return None;
        }
        let numbers: HashMap<usize, usize> = listing
            .entries
            .iter()
            .filter_map(|entry| match *entry {
                Entry::Row { session, number: Some(n), .. } => Some((session, n)),
                _ => None,
            })
            .collect();
        let frame = Frame::new(&input, self.window_width, self.hints_showing(cx), window, cx);
        let glass = input.glass.clone();
        let ui = frame.ui;
        let mut tabs: Vec<AnyElement> = Vec::new();
        // A newly selected terminal's tab is brought into view (once, so the row can be scrolled away).
        let active_id = input.active_id();
        if self.tabs.scrolled_to != active_id
            && let Some(at) =
                view.sets.get(current).and_then(|set| set.members.iter().position(|&m| m == input.active))
        {
            // The glass's clip probe, when there is one, is the row's first child.
            self.tabs.scroll.scroll_to_item(at + usize::from(glass.is_some()));
            self.tabs.scrolled_to = active_id;
        }
        if let Some(set) = view.sets.get(current) {
            // Each tab is a box of its own, so nothing is drawn between them.
            for (at, &ix) in set.members.iter().enumerate() {
                tabs.push(self.tab(
                    &frame,
                    infos,
                    &numbers,
                    (at, set.members.len()),
                    ix,
                    glass.as_ref(),
                    window,
                    cx,
                ));
            }
        }
        let alert = others_need_you(&view.sets, current, |ix| Urgency::from(glyph_for(&infos[ix])));
        let switcher = switch.then(|| self.render_switcher(&view, infos, &frame, alert, cx));
        let chip = self.render_profile_chip(cx);
        let group = view.looks.get(current).and_then(|look| look.group);
        // Under the terminal's title row, which is the same as with the sidebar (the lights, the
        // breadcrumb, the buttons), across the terminal's column and with no fill, as the row has none.
        let bar = div()
            .id("tab-bar")
            .absolute()
            .top(px(style::TITLE_ROW_H))
            .left_0()
            .right_0()
            .flex()
            .items_center()
            .gap_2()
            .h(px(style::TAB_H))
            // The terminal's surface stands on the window's 12 pt edge: the tabs line up with it.
            .px(px(style::EDGE_INSET))
            // Not part of the title bar: a drag here doesn't move the window, nor a double-click zoom it.
            .when_some(chip, |d, chip| d.child(chip).child(style::tab_divider(ui, PART_RULE_H)))
            .when_some(switcher, |d, switcher| d.child(switcher).child(style::tab_divider(ui, PART_RULE_H)))
            .child(
                div()
                    .id("tab-row")
                    .role(Role::TabList)
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    // Tabs give way down to their least width, then the row scrolls sideways.
                    .overflow_x_scroll()
                    .track_scroll(&self.tabs.scroll)
                    .children(glass.as_ref().map(|glass| clip_probe(glass, TAB_ROW_CLIP)))
                    .children(tabs),
            )
            // The + stays at the bar's right end, however many tabs there are.
            .child(self.new_tab_button(group, cx))
            .into_any_element();
        Some(bar)
    }

    /// A set's key (its group's), and its colour.
    fn set_look(&self, listing: &Listing, set: &TabSet, cx: &App) -> SetLook {
        let band = set.band.and_then(|band| listing.bands.get(band));
        let key = band.map_or_else(|| "ungrouped".to_string(), |band| band.key());
        let group = band.map(|band| band.group);
        let tokens = &store_in(&self.input.profile, cx).tokens;
        let paint = group.and_then(|group| self.view.groups.get(group)).and_then(|group| group.paint());
        let color = paint.map(|color| crate::theme_bridge::hsla(color.color(tokens)));
        let wash = paint.map(|color| crate::theme_bridge::hsla(color.wash(tokens)));
        SetLook { key, color, wash, group, pinned: band.is_some_and(|band| band.pinned) }
    }

    /// Open set `set`: the terminal last selected in it, else its first; an empty group gets a new
    /// terminal.
    pub(super) fn open_set(
        &mut self,
        bar_set: &TabSet,
        key: &str,
        group: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let last = self.tabs.last.get(key).and_then(|&id| self.input.find(id));
        let target =
            last.filter(|ix| bar_set.members.contains(ix)).or_else(|| bar_set.members.first().copied());
        match (target, group) {
            (Some(ix), _) => {
                cx.emit(SidebarEvent::Activate { id: self.input.rows[ix].id, then_focus_list: false })
            }
            (None, Some(group)) => cx.emit(SidebarEvent::NewTerminal(NewTerminal::InGroup(group))),
            (None, None) => {}
        }
    }

    /// Open the set at `at` of the list as it is now.
    pub(super) fn open_set_at(&mut self, at: usize, cx: &mut Context<Self>) {
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        let (Some(set), Some(look)) = (view.sets.get(at), view.looks.get(at)) else { return };
        self.open_set(set, &look.key, look.group, cx);
    }

    /// ⌃⌥] and ⌃⌥[: the next or previous group, in the switcher's order with the terminals in no
    /// group last. Both layouts.
    pub(crate) fn step_group(&mut self, forward: bool, cx: &mut Context<Self>) {
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        let Some(current) = view.current else { return };
        if let Some(at) = step_set(&view.sets, current, forward) {
            self.open_set(&view.sets[at], &view.looks[at].key, view.looks[at].group, cx);
        }
    }

    /// ⌃⌥1–9: the group with that number in the switcher's list.
    pub(crate) fn go_to_group(&mut self, number: u8, cx: &mut Context<Self>) {
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        if let Some(at) = set_numbered(&view.sets, usize::from(number)) {
            self.open_set(&view.sets[at], &view.looks[at].key, view.looks[at].group, cx);
        }
    }

    /// One terminal's tab: its tile and name, its menu and its close ×. On glass a tab is a glass
    /// of its own.
    #[allow(clippy::too_many_arguments)]
    fn tab(
        &self,
        frame: &Frame,
        infos: &[RowInfo],
        numbers: &HashMap<usize, usize>,
        (position, of): (usize, usize),
        ix: usize,
        glass: Option<&std::rc::Rc<crate::platform::vibrancy::Glass>>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(info) = infos.get(ix) else { return div().into_any_element() };
        let (ui, id) = (frame.ui, info.id);
        let selected = ix == self.input.active;
        let hint = numbers.get(&ix).filter(|_| frame.hints).map(|&n| SharedString::from(keycap(n)));
        let group = SharedString::from(style::TAB_GROUP);
        let close = (!frame.hints).then(|| self.tab_close(info, &group, ui, cx));
        let spoken =
            spoken::tab_label(&info.name, spoken::state_word(info.state), selected, position + 1, of);
        let name_room = style::TAB_NAME_W - if frame.hints { HINT_W } else { 0.0 };
        let tab = style::terminal_tab(
            ("tab", info.id as u64),
            ui,
            TerminalTab {
                tile: tile_for(info, frame, selected, style::TILE_SMALL, cx),
                name: info.name.clone(),
                selected,
                hint,
                close,
                // The keycap, while ⌘ is held, takes its room from the name's end; under the pointer
                // the × does.
                name_fade: frame.fade(&info.name, TAB_NAME_REM, FontWeight::MEDIUM, name_room, window),
                name_hover: frame.fade(
                    &info.name,
                    TAB_NAME_REM,
                    FontWeight::MEDIUM,
                    name_room - style::CLOSE_GLYPH_ROOM,
                    window,
                ),
            },
        )
        .when(glass.is_some() && selected, |d| d.bg(ui.selected_row.opacity(GLASS_SELECTED)))
        .children(glass.map(|glass| {
            probe(
                glass,
                glass_key(GlassKind::Tab, info.id as u64),
                Insets::NONE,
                style::CARD_ROW_RADIUS,
                Some(TAB_ROW_CLIP),
                false,
            )
        }))
        .group(group)
        .role(Role::Tab)
        .aria_label(spoken)
        .aria_selected(selected)
        .on_click(
            cx.listener(move |_, _, _, cx| cx.emit(SidebarEvent::Activate { id, then_focus_list: false })),
        );
        self.with_row_menu(tab, info, cx).into_any_element()
    }

    /// The × at a tab's right end, shown while the pointer is over the tab (the open one too).
    fn tab_close(
        &self,
        info: &RowInfo,
        group: &SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = info.id;
        // No tooltip while a close question is up: the pointer still rests on the ×.
        let asking = self.input.asking;
        // Laid over the tab's right end, its glyph where the tab's padding starts, over the faded end
        // of a long name; it shows under the pointer (a style that only paints, as the sidebar row's ×).
        div()
            .absolute()
            .top(px((style::TAB_H - style::CONTROL_H_SM) / 2.0))
            .right(px(CLOSE_RIGHT))
            .size(px(style::CONTROL_H_SM))
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .child(style::hit_area(
                ("close-hit", info.id as u64),
                style::CONTROL_H_SM,
                style::close_x(("close", info.id as u64), ui, style::SelectedFill::Plain),
                |hit| {
                    hit.role(Role::Button)
                        .aria_label("Close terminal")
                        .when(!asking, |d| {
                            d.tooltip(|window, cx| Tooltip::new("Close terminal").build(window, cx))
                        })
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.stop_propagation();
                            cx.emit(SidebarEvent::Close(id));
                        }))
                },
            ))
            .into_any_element()
    }

    /// The + after a set's tabs: a new terminal in it (in its group, or where the selected one is).
    /// A right-click offers the other shells (profiles), as the sidebar's + does; one opens in the
    /// selected terminal's set too.
    fn new_tab_button(&self, group: Option<u32>, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.input.profile, cx);
        let button = style::icon_button(
            "tab-new",
            ui,
            IconName::Plus,
            keymap::with_key("New terminal", ActionId::NewTerminal),
        )
        .on_click(cx.listener(move |_, _, _, cx| {
            cx.emit(SidebarEvent::NewTerminal(group.map_or(NewTerminal::Plain, NewTerminal::InGroup)))
        }));
        let this = cx.weak_entity();
        let on_shell: super::shells::OnShell = std::rc::Rc::new(move |key, _, cx| {
            this.update(cx, |_, cx| cx.emit(SidebarEvent::NewTerminal(NewTerminal::Shell(key)))).ok();
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .child(super::shells::with_shell_menu("tab-new-shells", button, on_shell))
            .into_any_element()
    }
}
