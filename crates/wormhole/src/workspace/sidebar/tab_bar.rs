//! The terminals as tabs, instead of the sidebar (Settings → Show terminals in → Tabs), as RF-02 draws
//! it: one 36 pt row with the traffic lights, the profile chip (while there are two profiles or
//! more), the group switcher (`switcher`), that group's terminals as tabs and the +. The sets of tabs
//! are the list's groups and the terminals in no group (core `sidebar::tabs`); ⌃⌥[ ⌃⌥] and ⌃⌥1–9
//! move between them in either layout.
use std::collections::HashMap;

use gpui_kit::component::tooltip::Tooltip;
use wormhole_core::sidebar::Urgency;
use wormhole_core::sidebar::hints::keycap;
use wormhole_core::sidebar::spoken;
use wormhole_core::sidebar::tabs::{TabSet, current_set, others_need_you, set_numbered, step_set, tab_sets};

use super::super::chrome::{PANEL_TOGGLE_SLOT, TitleDrag as _, lights_end, traffic_light_position};
use super::super::glass::{GlassKind, Insets, TAB_ROW_CLIP, glass_key};
use super::rows::{Frame, glyph_for, tile_for};
use super::*;
use crate::style::TerminalTab;

/// The room between the traffic lights and the first thing after them on the tab bar.
const AFTER_LIGHTS: f32 = 12.0;
/// How much of the selected tab's fill lies over its glass.
const GLASS_SELECTED: f32 = 0.7;
/// The hairlines between the bar's parts and between tabs.
const PART_RULE_H: f32 = 16.0;
const TAB_RULE_H: f32 = 14.0;

/// What the tab bar remembers between frames.
#[derive(Default)]
pub(in crate::workspace) struct TabBarState {
    /// The terminals are in tabs (Settings → Show terminals in), in every window.
    pub(in crate::workspace) on: bool,
    /// The terminal last selected in each set, by its key: where opening the set goes back to.
    last: HashMap<String, i64>,
    /// The height of the row the traffic lights were last centred on.
    lights_row: Option<f32>,
    /// The group switcher being open: its search field.
    pub(super) switcher: Option<super::switcher::Switcher>,
}

/// One set as the bar draws it.
pub(super) struct SetLook {
    pub(super) key: String,
    pub(super) color: Option<Hsla>,
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

impl Workspace {
    /// Whether the sidebar is on screen: wanted, and the terminals aren't in tabs.
    pub(in crate::workspace) fn sidebar_shown(&self) -> bool {
        self.panes.sidebar_open && !self.sidebar.tabs.on
    }

    /// Whether the terminals are in tabs across the top.
    pub(in crate::workspace) fn tabs_on_top(&self) -> bool {
        self.sidebar.tabs.on
    }

    /// Read where the terminals are listed from the settings (once a frame, before anything is laid
    /// out), and centre the traffic lights on the row they sit in.
    pub(in crate::workspace) fn sync_tab_layout(&mut self, window: &mut Window, cx: &App) {
        self.sidebar.tabs.on =
            settings_in(&self.profile, cx).terminals_in == wormhole_core::sidebar::TerminalsIn::Tabs;
        let row = style::TITLE_ROW_H;
        if self.sidebar.tabs.lights_row != Some(row) {
            self.sidebar.tabs.lights_row = Some(row);
            window.set_traffic_light_position(traffic_light_position(row));
        }
    }

    /// The sets of the list as it is now, each with its look, and the current one; the terminal
    /// last selected in the current one is remembered, so coming back to the set lands there.
    pub(super) fn set_view(&mut self, listing: &Listing, cx: &App) -> SetView {
        let sets = tab_sets(listing);
        let current = current_set(&sets, self.active);
        let looks: Vec<SetLook> = sets.iter().map(|set| self.set_look(listing, set, cx)).collect();
        if let (Some(look), Some(session)) =
            (current.and_then(|at| looks.get(at)), self.sessions.get(self.active))
        {
            self.sidebar.tabs.last.insert(look.key.clone(), session.id);
        }
        SetView { sets, looks, current }
    }

    /// The tab bar.
    pub(in crate::workspace) fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let view = self.set_view(&listing, cx);
        let current = view.current.unwrap_or(0);
        let numbers: HashMap<usize, usize> = listing
            .entries
            .iter()
            .filter_map(|entry| match *entry {
                Entry::Row { session, number: Some(n), .. } => Some((session, n)),
                _ => None,
            })
            .collect();
        let frame = Frame::new(self, self.window_width, window, cx);
        let glass = self.glass_on(cx);
        let ui = frame.ui;
        let selected_tab =
            view.sets.get(current).and_then(|set| set.members.iter().position(|&m| m == self.active));
        let mut tabs: Vec<AnyElement> = Vec::new();
        if let Some(set) = view.sets.get(current) {
            for (at, &ix) in set.members.iter().enumerate() {
                // A hairline between two tabs that are neither the open one: the open tab's fill is its edge.
                if at > 0 {
                    let hidden = selected_tab == Some(at) || selected_tab == Some(at - 1);
                    let rule = style::tab_divider(ui, TAB_RULE_H);
                    tabs.push(rule.when(hidden, |d| d.bg(gpui_kit::transparent_black())).into_any_element());
                }
                tabs.push(self.tab(&frame, &infos, &numbers, (at, set.members.len()), ix, glass, cx));
            }
        }
        let alert = others_need_you(&view.sets, current, |ix| Urgency::from(glyph_for(&infos[ix])));
        let switcher = self.render_switcher(&view, &infos, &frame, alert, cx);
        let chip = self.render_profile_chip(cx);
        let group = view.looks.get(current).and_then(|look| look.group);
        div()
            .id("tab-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(px(style::TITLE_ROW_H))
            .pl(px(lights_end(window) + AFTER_LIGHTS))
            .pr(px(PANEL_TOGGLE_SLOT))
            .bg(if glass { gpui_kit::transparent_black() } else { ui.panel })
            .when(!glass, |d| d.rule_bottom(ui))
            .title_drag()
            .when_some(chip, |d, chip| d.child(chip).child(style::tab_divider(ui, PART_RULE_H)))
            .child(switcher)
            .child(style::tab_divider(ui, PART_RULE_H))
            .child(
                div()
                    .id("tab-row")
                    .role(Role::TabList)
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .when(glass, |d| d.child(self.glass_clip_probe(TAB_ROW_CLIP)))
                    .children(tabs)
                    .child(self.new_tab_button(group, cx)),
            )
            .into_any_element()
    }

    /// A set's key (its group's), and its colour.
    fn set_look(&self, listing: &Listing, set: &TabSet, cx: &App) -> SetLook {
        let band = set.band.and_then(|band| listing.bands.get(band));
        let key = band.map_or_else(|| "ungrouped".to_string(), |band| band.key());
        let group = band.map(|band| band.group);
        let tokens = &store_in(&self.profile, cx).tokens;
        let color = group
            .and_then(|group| self.sidebar.view.groups.get(group))
            .and_then(|group| group.color())
            .map(|color| crate::theme_bridge::hsla(color.color(tokens)));
        SetLook { key, color, group, pinned: band.is_some_and(|band| band.pinned) }
    }

    /// Open set `set`: the terminal last selected in it, else its first; an empty group gets a new
    /// terminal.
    pub(super) fn open_set(
        &mut self,
        bar_set: &TabSet,
        key: &str,
        group: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let last = self.sidebar.tabs.last.get(key).and_then(|&id| self.find_session(id));
        let target =
            last.filter(|ix| bar_set.members.contains(ix)).or_else(|| bar_set.members.first().copied());
        match (target, group) {
            (Some(ix), _) => self.activate(ix, window, cx),
            (None, Some(group)) => self.new_session_in_group(group, window, cx),
            (None, None) => {}
        }
    }

    /// Open the set at `at` of the list as it is now.
    pub(super) fn open_set_at(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let view = self.set_view(&listing, cx);
        let (Some(set), Some(look)) = (view.sets.get(at), view.looks.get(at)) else { return };
        self.open_set(set, &look.key, look.group, window, cx);
    }

    /// ⌃⌥] and ⌃⌥[: the next or previous group, in the switcher's order with the terminals in no
    /// group last. Both layouts.
    pub(in crate::workspace) fn step_group(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let view = self.set_view(&listing, cx);
        let Some(current) = view.current else { return };
        if let Some(at) = step_set(&view.sets, current, forward) {
            self.open_set(&view.sets[at], &view.looks[at].key, view.looks[at].group, window, cx);
        }
    }

    /// ⌃⌥1–9: the group with that number in the switcher's list.
    pub(in crate::workspace) fn go_to_group(
        &mut self,
        number: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let view = self.set_view(&listing, cx);
        if let Some(at) = set_numbered(&view.sets, usize::from(number)) {
            self.open_set(&view.sets[at], &view.looks[at].key, view.looks[at].group, window, cx);
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
        glass: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(info) = infos.get(ix) else { return div().into_any_element() };
        let ui = frame.ui;
        let selected = ix == self.active;
        let hint = numbers.get(&ix).filter(|_| frame.hints).map(|&n| SharedString::from(keycap(n)));
        let group = SharedString::from(style::TAB_GROUP);
        let close = (!frame.hints).then(|| self.tab_close(info, &group, ui, cx));
        let spoken =
            spoken::tab_label(&info.name, spoken::state_word(info.state), selected, position + 1, of);
        let tab = style::terminal_tab(
            ("tab", info.id as u64),
            ui,
            TerminalTab {
                tile: tile_for(info, frame, selected, style::TILE_SMALL, cx),
                name: info.name.clone(),
                selected,
                hint,
                close,
            },
        )
        .when(glass && selected, |d| d.bg(ui.selected_row.opacity(GLASS_SELECTED)))
        .when(glass, |d| {
            d.child(self.glass_probe(
                glass_key(GlassKind::Tab, info.id as u64),
                Insets::NONE,
                style::CARD_ROW_RADIUS,
                Some(TAB_ROW_CLIP),
            ))
        })
        .group(group)
        .role(Role::Tab)
        .aria_label(spoken)
        .aria_selected(selected)
        .on_click(cx.listener(move |this, _, window, cx| this.activate(ix, window, cx)));
        self.with_row_menu(tab, info, cx).into_any_element()
    }

    /// The × of a tab, while the pointer is over it (the open one too): no width and no gap before
    /// it otherwise, so the name has the room, and it takes its slot from the name when shown. The
    /// tab itself keeps its width either way.
    fn tab_close(
        &self,
        info: &RowInfo,
        group: &SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ix = info.ix;
        // No tooltip while a close question is up: the pointer still rests on the ×.
        let asking = self.closing_session_prompt || self.closing_prompt;
        div()
            .flex_none()
            .overflow_hidden()
            .w(px(0.0))
            .group_hover(group.clone(), |s| s.w(px(style::CONTROL_H_SM)).ml_2())
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
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.request_close_session(ix, window, cx);
                        }))
                },
            ))
            .into_any_element()
    }

    /// The + after a set's tabs: a new terminal in it (in its group, or where the selected one is).
    /// A right-click offers the other shells (profiles), as the sidebar's + does; one opens in the
    /// selected terminal's set too.
    fn new_tab_button(&self, group: Option<u32>, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let button = style::icon_button(
            "tab-new",
            ui,
            IconName::Plus,
            keymap::with_key("New terminal", ActionId::NewTerminal),
        )
        .on_click(cx.listener(move |this, _, window, cx| match group {
            Some(group) => this.new_session_in_group(group, window, cx),
            None => this.new_session(window, cx),
        }));
        div()
            .flex()
            .flex_none()
            .items_center()
            .child(self.with_shell_menu("tab-new-shells", button, cx))
            .into_any_element()
    }
}
