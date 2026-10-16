//! The terminals as tabs, instead of the sidebar (Settings → Show terminals in → Tabs), in one of
//! three layouts (`TabLayout`). The sets of tabs are the list's bands (core `sidebar::tabs`): the
//! selected terminal's set shows its tabs, the others are chips above them (two rows), piles beside
//! them that fan out under the pointer (stacks), or segments of a bar along the bottom. A tab is
//! its terminal's row in short: the status tile, the name and the second line, the row's menu and
//! its close ×.
use std::collections::HashMap;

use wormhole_core::folder_color::FolderColor;
use wormhole_core::sidebar::TabLayout;
use wormhole_core::sidebar::hints::keycap;
use wormhole_core::sidebar::tabs::{TabSet, current_set, tab_sets};
use wormhole_core::sidebar::text::place_detail;

use super::super::chrome::{PANEL_TOGGLE_SLOT, TitleDrag as _, lights_end, traffic_light_position};
use super::super::folder_color;
use super::rows::{Frame, PLACE_CHARS, SecondLine, second_line, tile_for};
use super::*;
use crate::settings::SettingsStore;
use crate::style::{GroupChip, PileFan, StatusGlyph, StatusTile, TabPile, TerminalTab, TileFace};

/// The room between the traffic lights and the first thing after them on a tab bar.
const AFTER_LIGHTS: f32 = 12.0;
/// The stacks layout's one row: a tab and the room round it.
const STACKS_H: f32 = 60.0;
/// The two-row layout's tab row: its card's padding, and the room under it.
const TAB_ROW_PAD: f32 = 6.0;
const TAB_ROW_BOTTOM: f32 = 8.0;

/// What the tab bars remember between frames.
#[derive(Default)]
pub(in crate::workspace) struct TabBarState {
    /// The layout every window shows its tabs in now; `None` while the terminals are in the sidebar.
    pub(in crate::workspace) layout: Option<TabLayout>,
    /// The pile under the pointer (stacks), and the one it last left, by their sets' keys.
    hover: Option<String>,
    left: Option<String>,
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
        self.panes.sidebar_open && self.sidebar.tabs.layout.is_none()
    }

    /// Whether the terminals are in tabs across the top (two rows, stacks).
    pub(in crate::workspace) fn tabs_on_top(&self) -> bool {
        self.sidebar.tabs.layout.is_some_and(TabLayout::on_top)
    }

    /// Read where the terminals are listed from the settings (once a frame, before anything is laid
    /// out), and centre the traffic lights on the row they sit in.
    pub(in crate::workspace) fn sync_tab_layout(&mut self, window: &mut Window, cx: &App) {
        let s = settings(cx);
        self.sidebar.tabs.layout =
            (s.terminals_in == wormhole_core::sidebar::TerminalsIn::Tabs).then_some(s.tab_layout);
        let row = self.top_row_h();
        if self.sidebar.tabs.lights_row != Some(row) {
            self.sidebar.tabs.lights_row = Some(row);
            window.set_traffic_light_position(traffic_light_position(row));
        }
    }

    /// How tall the window's top row is, the one the traffic lights and the panel toggle are
    /// centred on: the stacks layout's tab bar, else a title row.
    pub(in crate::workspace) fn top_row_h(&self) -> f32 {
        match self.sidebar.tabs.layout {
            Some(TabLayout::Stacks) => STACKS_H,
            _ => style::TITLE_ROW_H,
        }
    }

    /// The tab bar of `layout`.
    pub(in crate::workspace) fn render_tab_bar(
        &mut self,
        layout: TabLayout,
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
        let bar =
            Bar { sets: &sets, looks: &looks, current, infos: &infos, numbers: &numbers, frame: &frame };
        match layout {
            TabLayout::TwoRows => self.two_rows(&bar, window, cx),
            TabLayout::Stacks => self.stacks(&bar, window, cx),
            TabLayout::Bottom => self.bottom_bar(&bar, cx),
        }
    }

    /// A set's key (its band's), colour (its group's or folder's) and group.
    fn set_look(&self, listing: &Listing, set: &TabSet, cx: &App) -> SetLook {
        let band = set.band.and_then(|band| listing.bands.get(band));
        let key = band.map_or_else(|| "ungrouped".to_string(), |band| band.key());
        let group = band.and_then(|band| band.group);
        let tokens = &cx.global::<SettingsStore>().tokens;
        let color = match (group, band.and_then(|band| band.folder.as_deref())) {
            (Some(group), _) => self
                .sidebar
                .view
                .groups
                .get(group)
                .and_then(|g| FolderColor::from_key(&g.color))
                .map(|color| folder_color::band_tint_in(color, tokens).color),
            (None, Some(folder)) => self.band_tint(folder, cx).map(|(_, tint)| tint.color),
            (None, None) => None,
        };
        SetLook { key, color, group }
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

    /// One terminal's tab: its tile, name and second line, its menu and its close ×.
    fn tab(&self, bar: &Bar, ix: usize, fixed: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(info) = bar.infos.get(ix) else { return div().into_any_element() };
        let frame = bar.frame;
        let ui = frame.ui;
        let selected = ix == self.active;
        let branch = info.branch.as_deref().filter(|_| frame.show_branch);
        let whereabouts = || place_detail(info.cwd.as_deref(), None, branch, home_dir(), PLACE_CHARS);
        let SecondLine { text, tone, mono, .. } = second_line(info, false, whereabouts);
        let hint = bar.numbers.get(&ix).filter(|_| frame.hints).map(|&n| SharedString::from(keycap(n)));
        let group = SharedString::from("terminal-tab");
        let tab = style::terminal_tab(
            ("tab", info.id as u64),
            ui,
            TerminalTab {
                tile: tile_for(info, frame, selected, false, cx),
                name: info.name.clone(),
                detail: text.map(SharedString::from),
                tone,
                detail_font: mono.then(|| frame.code_font.clone()),
                selected,
                hint,
                fixed,
            },
        )
        .group(group.clone())
        .role(Role::Tab)
        .aria_label(info.name.clone())
        .aria_selected(selected)
        .on_click(cx.listener(move |this, _, window, cx| this.activate(ix, window, cx)))
        .when(!frame.hints, |d| d.child(self.close_button(info, selected, &group, ui, cx)));
        self.with_row_menu(tab, info, cx).into_any_element()
    }

    /// The + after a set's tabs: a new terminal in it (in its group, or where the selected one is).
    fn new_tab_button(&self, look: &SetLook, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let group = look.group;
        style::icon_button("tab-new", ui, IconName::Plus, "New terminal")
            .on_click(cx.listener(move |this, _, window, cx| match group {
                Some(group) => this.new_session_in_group(group, window, cx),
                None => this.new_session(window, cx),
            }))
            .into_any_element()
    }

    /// Settings, at a tab bar's end (the sidebar's gear isn't on screen).
    fn settings_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        style::icon_button(
            "tab-settings",
            ui,
            IconName::Settings,
            keymap::with_key("Settings", ActionId::Settings),
        )
        .in_title_row()
        .on_click(cx.listener(|this, _, window, cx| this.toggle_settings(window, cx)))
        .into_any_element()
    }

    /// New group, while the list shows the person's groups.
    fn new_group_button(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        (self.sidebar.view.grouping == Grouping::Custom).then(|| {
            style::icon_button("tab-new-group", ui(cx), IconName::FolderPlus, "New group")
                .in_title_row()
                .on_click(cx.listener(|this, _, window, cx| this.new_group(window, cx)))
                .into_any_element()
        })
    }

    /// Two rows across the top: the sets as chips beside the lights, the selected set's tabs on a
    /// card under them.
    fn two_rows(&self, bar: &Bar, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = bar.frame.ui;
        let chips: Vec<AnyElement> = bar
            .sets
            .iter()
            .zip(bar.looks)
            .enumerate()
            .map(|(at, (set, look))| {
                let alert = Self::set_alert(bar, set);
                let (set, key, group) = (set.clone(), look.key.clone(), look.group);
                style::group_chip(
                    ElementId::Name(SharedString::from(format!("tab-chip-{}", look.key))),
                    ui,
                    GroupChip {
                        color: look.color,
                        name: set.label.clone().into(),
                        count: set.members.len(),
                        selected: at == bar.current,
                        alert,
                    },
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_set(&set, &key, group, window, cx)),
                )
                .into_any_element()
            })
            .collect();
        let current = &bar.sets[bar.current];
        let look = &bar.looks[bar.current];
        let tint = look.color.unwrap_or(ui.border);
        let tabs: Vec<AnyElement> = current.members.iter().map(|&ix| self.tab(bar, ix, false, cx)).collect();
        div()
            .id("tab-bar")
            .flex()
            .flex_col()
            .flex_none()
            .bg(ui.panel)
            .rule_bottom(ui)
            .child(
                div()
                    .id("tab-sets")
                    .h(px(style::TITLE_ROW_H))
                    .flex()
                    .items_center()
                    .gap_1()
                    .pl(px(lights_end(window) + AFTER_LIGHTS))
                    .pr(px(PANEL_TOGGLE_SLOT))
                    .title_drag()
                    .children(chips)
                    .children(self.new_group_button(cx))
                    .child(div().flex_1())
                    .child(self.settings_button(cx)),
            )
            .child(
                div().px(px(style::CARD_MARGIN)).pb(px(TAB_ROW_BOTTOM)).child(
                    div()
                        .id("tab-row")
                        .role(Role::TabList)
                        .flex()
                        .items_center()
                        .gap_1()
                        .p(px(TAB_ROW_PAD))
                        .rounded(px(style::CARD_RADIUS))
                        .overflow_hidden()
                        .bg(ui.band)
                        .outlined(ui, tint.opacity(TINT_OUTLINE))
                        .children(tabs)
                        .child(self.new_tab_button(look, cx)),
                ),
            )
            .into_any_element()
    }

    /// One row across the top: the selected set's name and tabs, and the other sets piled up.
    fn stacks(&self, bar: &Bar, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = bar.frame.ui;
        let current = &bar.sets[bar.current];
        let look = &bar.looks[bar.current];
        let tabs: Vec<AnyElement> = current.members.iter().map(|&ix| self.tab(bar, ix, true, cx)).collect();
        let piles: Vec<AnyElement> = bar
            .sets
            .iter()
            .zip(bar.looks)
            .enumerate()
            .filter(|(at, _)| *at != bar.current)
            .map(|(_, (set, look))| self.pile(bar, set, look, cx))
            .collect();
        div()
            .id("tab-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(px(STACKS_H))
            .pl(px(lights_end(window) + AFTER_LIGHTS))
            .pr(px(PANEL_TOGGLE_SLOT))
            .bg(ui.panel)
            .rule_bottom(ui)
            .title_drag()
            .child(style::set_label(ui, look.color, current.label.clone()))
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
                    .children(tabs)
                    .child(self.new_tab_button(look, cx)),
            )
            .when(!piles.is_empty(), |d| d.child(vertical_rule(ui)).children(piles))
            .children(self.new_group_button(cx))
            .child(self.settings_button(cx))
            .into_any_element()
    }

    /// A set that isn't the selected one, piled up: the tile of its loudest terminal, its name and
    /// how it is doing. The cards behind fan out under the pointer; a click opens it.
    fn pile(&self, bar: &Bar, set: &TabSet, look: &SetLook, cx: &mut Context<Self>) -> AnyElement {
        let ui = bar.frame.ui;
        let lead = Self::loudest(bar, set);
        let tile = match lead.and_then(|ix| bar.infos.get(ix)) {
            Some(info) => tile_for(info, bar.frame, false, false, cx),
            None => style::status_tile(
                ElementId::Name(SharedString::from(format!("pile-empty-{}", look.key))),
                ui,
                StatusTile {
                    face: TileFace::Terminal,
                    glyph: StatusGlyph::Idle,
                    selected: false,
                    mini: false,
                },
                cx,
            ),
        };
        let (summary, summary_color) = Self::set_summary(bar, set, ui);
        let tabs = &self.sidebar.tabs;
        let fan = if tabs.hover.as_deref() == Some(look.key.as_str()) {
            PileFan::Opening
        } else if tabs.left.as_deref() == Some(look.key.as_str()) {
            PileFan::Closing
        } else {
            PileFan::Rest
        };
        let (set, key, group, hover_key) = (set.clone(), look.key.clone(), look.group, look.key.clone());
        style::tab_pile(
            ElementId::Name(SharedString::from(format!("tab-pile-{}", look.key))),
            ui,
            TabPile {
                color: look.color,
                name: set.label.clone().into(),
                summary: summary.into(),
                summary_color,
                lead: tile,
                fan,
            },
        )
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            let tabs = &mut this.sidebar.tabs;
            if *hovered {
                tabs.hover = Some(hover_key.clone());
            } else if tabs.hover.as_deref() == Some(hover_key.as_str()) {
                tabs.hover = None;
                tabs.left = Some(hover_key.clone());
            }
            cx.notify();
        }))
        .on_click(cx.listener(move |this, _, window, cx| this.open_set(&set, &key, group, window, cx)))
        .into_any_element()
    }

    /// A bar along the bottom: every set a segment, the selected one opened into its tabs.
    fn bottom_bar(&self, bar: &Bar, cx: &mut Context<Self>) -> AnyElement {
        let ui = bar.frame.ui;
        let segments: Vec<AnyElement> = bar
            .sets
            .iter()
            .zip(bar.looks)
            .enumerate()
            .map(|(at, (set, look))| {
                let id = ElementId::Name(SharedString::from(format!("tab-segment-{}", look.key)));
                let segment = style::tab_segment(id, ui, look.color, set.label.clone());
                if at == bar.current {
                    let tabs: Vec<AnyElement> =
                        set.members.iter().map(|&ix| self.tab(bar, ix, true, cx)).collect();
                    segment
                        .flex_shrink_1()
                        .child(
                            div()
                                .id("tab-row")
                                .role(Role::TabList)
                                .flex()
                                .items_center()
                                .gap_1()
                                .min_w_0()
                                .overflow_hidden()
                                .children(tabs),
                        )
                        .child(self.new_tab_button(look, cx))
                        .into_any_element()
                } else {
                    let lead = Self::loudest(bar, set).and_then(|ix| bar.infos.get(ix));
                    let tile = lead.map(|info| tile_for(info, bar.frame, false, true, cx));
                    let (_, summary_color) = Self::set_summary(bar, set, ui);
                    let (set, key, group) = (set.clone(), look.key.clone(), look.group);
                    let count = set.members.len();
                    segment
                        .flex_none()
                        .gap_2()
                        .pr_3()
                        .cursor_pointer()
                        .hover(|d| d.bg(ui.band_hover))
                        .children(tile)
                        .child(
                            div()
                                .text_caption()
                                .tabular_nums()
                                .text_color(summary_color)
                                .child(count.to_string()),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_set(&set, &key, group, window, cx)
                        }))
                        .into_any_element()
                }
            })
            .collect();
        div()
            .id("tab-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .px(px(style::CARD_MARGIN))
            .py(px(TAB_ROW_PAD))
            .bg(ui.panel)
            .rule_top(ui)
            .children(segments)
            .children(self.new_group_button(cx))
            .child(div().flex_1())
            .child(self.settings_button(cx))
            .into_any_element()
    }

    /// The terminal of `set` that most wants attention: needs you, then failed, working, the bell,
    /// new output, running, finished, idle.
    fn loudest(bar: &Bar, set: &TabSet) -> Option<usize> {
        let rank = |glyph: StatusGlyph| match glyph {
            StatusGlyph::NeedsYou => 0,
            StatusGlyph::Failed => 1,
            StatusGlyph::Working => 2,
            StatusGlyph::Bell => 3,
            StatusGlyph::Terminal { unread: true } => 4,
            StatusGlyph::Running | StatusGlyph::Terminal { unread: false } => 5,
            StatusGlyph::Done => 6,
            StatusGlyph::Idle => 7,
            StatusGlyph::Shell => 8,
        };
        set.members
            .iter()
            .copied()
            .min_by_key(|&ix| bar.infos.get(ix).map_or(9, |info| rank(super::rows::glyph_for(info))))
    }

    /// How a set is doing in words, and their colour: "3 · 1 waiting", "3 · 1 failed", "3 · 2
    /// working", else "3 terminals".
    fn set_summary(bar: &Bar, set: &TabSet, ui: UiColors) -> (String, Hsla) {
        let count = |state: RowState| set.members.iter().filter(|&&ix| bar.infos[ix].state == state).count();
        let n = set.members.len();
        match (count(RowState::NeedsYou), count(RowState::Failed), count(RowState::Working)) {
            (w, _, _) if w > 0 => (format!("{n} · {w} waiting"), ui.attention_text),
            (_, f, _) if f > 0 => (format!("{n} · {f} failed"), ui.danger_text),
            (_, _, k) if k > 0 => (format!("{n} · {k} working"), ui.muted),
            _ => (format!("{n} {}", if n == 1 { "terminal" } else { "terminals" }), ui.muted),
        }
    }

    /// The dot on a set's chip: amber while a terminal in it needs you, danger while one failed.
    fn set_alert(bar: &Bar, set: &TabSet) -> Option<Hsla> {
        let ui = bar.frame.ui;
        let any = |state: RowState| set.members.iter().any(|&ix| bar.infos[ix].state == state);
        if any(RowState::NeedsYou) {
            Some(ui.agent_attention)
        } else if any(RowState::Failed) {
            Some(ui.danger)
        } else {
            None
        }
    }
}

/// What every part of one frame's tab bar is drawn from.
struct Bar<'a> {
    sets: &'a [TabSet],
    looks: &'a [SetLook],
    current: usize,
    infos: &'a [RowInfo],
    /// The ⌘ numbers, by terminal.
    numbers: &'a HashMap<usize, usize>,
    frame: &'a Frame,
}

/// How strong a set's colour is round its tab row.
const TINT_OUTLINE: f32 = 0.45;

/// The hairline between the selected set's tabs and the piles.
#[track_caller]
fn vertical_rule(ui: UiColors) -> Div {
    div().flex_none().w(ui.border_width).h(px(RULE_H)).mx_2().bg(ui.border)
}

const RULE_H: f32 = 28.0;
