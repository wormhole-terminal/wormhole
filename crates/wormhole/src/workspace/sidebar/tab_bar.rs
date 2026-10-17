//! The terminals as tabs, instead of the sidebar (Settings → Show terminals in → Tabs): one row
//! across the top with the traffic lights, the selected terminal's group (its name, and a menu to
//! switch to another), that group's tabs and the +. The sets of tabs are the list's groups, and the
//! terminals in no group (core `sidebar::tabs`). A tab is its terminal in short: the status tile,
//! the name, its menu and its close ×.
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};

use super::super::focus::MenuBuilder;
use wormhole_core::sidebar::hints::keycap;
use wormhole_core::sidebar::tabs::{TabSet, current_set, tab_sets};

use super::super::chrome::{PANEL_TOGGLE_SLOT, TitleDrag as _, lights_end, traffic_light_position};
use super::super::glass::{GlassKind, Insets, TAB_ROW_CLIP, glass_key};
use super::rows::{Frame, tile_for};
use super::*;
use crate::style::TerminalTab;

/// The room between the traffic lights and the first thing after them on the tab bar.
const AFTER_LIGHTS: f32 = 12.0;
/// How much of the selected tab's fill lies over its glass.
const GLASS_SELECTED: f32 = 0.7;

/// What the tab bar remembers between frames.
#[derive(Default)]
pub(in crate::workspace) struct TabBarState {
    /// The terminals are in tabs (Settings → Show terminals in), in every window.
    pub(in crate::workspace) on: bool,
    /// The terminal last selected in each set, by its key: where opening the set goes back to.
    last: HashMap<String, i64>,
    /// The height of the row the traffic lights were last centred on.
    lights_row: Option<f32>,
}

/// One set as the bar draws it.
struct SetLook {
    key: String,
    color: Option<Hsla>,
    group: Option<u32>,
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
        self.sidebar.tabs.on = settings(cx).terminals_in == wormhole_core::sidebar::TerminalsIn::Tabs;
        let row = style::TITLE_ROW_H;
        if self.sidebar.tabs.lights_row != Some(row) {
            self.sidebar.tabs.lights_row = Some(row);
            window.set_traffic_light_position(traffic_light_position(row));
        }
    }

    /// The tab bar.
    pub(in crate::workspace) fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let sets = tab_sets(&listing);
        let current = current_set(&sets, self.active).unwrap_or(0);
        let looks: Vec<SetLook> = sets.iter().map(|set| self.set_look(&listing, set, cx)).collect();
        if let (Some(look), Some(session)) = (looks.get(current), self.sessions.get(self.active)) {
            self.sidebar.tabs.last.insert(look.key.clone(), session.id);
        }
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
        let tabs: Vec<AnyElement> = sets
            .get(current)
            .map(|set| {
                set.members.iter().map(|&ix| self.tab(&frame, &infos, &numbers, ix, glass, cx)).collect()
            })
            .unwrap_or_default();
        let look = looks.get(current);
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
            .child(self.set_switcher(&sets, &looks, current, ui, cx))
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
                    .child(self.new_tab_button(look.and_then(|look| look.group), cx)),
            )
            .into_any_element()
    }

    /// A set's key (its group's), and its colour.
    fn set_look(&self, listing: &Listing, set: &TabSet, cx: &App) -> SetLook {
        let band = set.band.and_then(|band| listing.bands.get(band));
        let key = band.map_or_else(|| "ungrouped".to_string(), |band| band.key());
        let group = band.map(|band| band.group);
        let tokens = &cx.global::<crate::settings::SettingsStore>().tokens;
        let color = group
            .and_then(|group| self.sidebar.view.groups.get(group))
            .and_then(|group| group.color())
            .map(|color| crate::theme_bridge::hsla(color.color(tokens)));
        SetLook { key, color, group }
    }

    /// The selected set's name, and a menu to open another.
    fn set_switcher(
        &self,
        sets: &[TabSet],
        looks: &[SetLook],
        current: usize,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let name = sets.get(current).map(|set| set.label.clone()).unwrap_or_default();
        let color = looks.get(current).and_then(|look| look.color).unwrap_or(ui.faint);
        let choices: Vec<(TabSet, String, Option<u32>)> =
            sets.iter().zip(looks).map(|(set, look)| (set.clone(), look.key.clone(), look.group)).collect();
        let this = cx.weak_entity();
        let menu: MenuBuilder = Rc::new(move |mut menu, _, _| {
            for (at, (set, key, group)) in choices.iter().cloned().enumerate() {
                let this = this.clone();
                menu = menu.item(
                    PopupMenuItem::new(format!("{} · {}", set.label, set.members.len()))
                        .checked(at == current)
                        .on_click(move |_, window, cx| {
                            this.update(cx, |ws, cx| ws.open_set(&set, &key, group, window, cx)).ok();
                        }),
                );
            }
            menu
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .child(style::color_dot(ui, color, style::COLOR_DOT_MENU))
            .child(div().max_w(px(160.0)).truncate().text_label().text_color(ui.text).child(name))
            .child(
                style::icon_button("tab-sets", ui, IconName::ChevronsUpDown, "Switch group")
                    .menu_trigger()
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |m, w, c| menu(m, w, c)),
            )
            .into_any_element()
    }

    /// Open set `set`: the terminal last selected in it, else its first; an empty group gets a new
    /// terminal.
    fn open_set(
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

    /// One terminal's tab: its tile and name, its menu and its close ×. On glass a tab is a glass
    /// of its own.
    fn tab(
        &self,
        frame: &Frame,
        infos: &[RowInfo],
        numbers: &HashMap<usize, usize>,
        ix: usize,
        glass: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(info) = infos.get(ix) else { return div().into_any_element() };
        let ui = frame.ui;
        let selected = ix == self.active;
        let hint = numbers.get(&ix).filter(|_| frame.hints).map(|&n| SharedString::from(keycap(n)));
        let group = SharedString::from("terminal-tab");
        let tab = style::terminal_tab(
            ("tab", info.id as u64),
            ui,
            TerminalTab {
                tile: tile_for(info, frame, selected, true, cx),
                name: info.name.clone(),
                selected,
                hint,
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
        .group(group.clone())
        .role(Role::Tab)
        .aria_label(info.name.clone())
        .aria_selected(selected)
        .on_click(cx.listener(move |this, _, window, cx| this.activate(ix, window, cx)))
        .when(!frame.hints, |d| d.child(self.close_button(info, selected, &group, ui, cx)));
        self.with_row_menu(tab, info, cx).into_any_element()
    }

    /// The + after a set's tabs: a new terminal in it (in its group, or where the selected one is).
    /// A right-click offers the other shells (profiles), as the sidebar's + does; one opens in the
    /// selected terminal's set too.
    fn new_tab_button(&self, group: Option<u32>, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
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
