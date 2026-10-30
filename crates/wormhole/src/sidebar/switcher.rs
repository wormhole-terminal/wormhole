//! The tab bar's group switcher and the popover it opens (RF-02 b): a search field ("Go to group or
//! terminal…"), the groups drawn as the sidebar's folded cards (their tiles and count, the open one
//! ringed, a pinned empty one saying "No terminals"), "Ungrouped" with its count, New group and
//! "Show all groups in the sidebar", and the keys' hint. A click on a card goes to the group; typing
//! lists the groups and terminals that match, and ↩ goes to the first.
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::popover::Popover;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;
use std::sync::Arc;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::panes::Rect;
use wormhole_core::view::picker::{Context as PickerContext, Note};
use wormhole_core::view::sidebar::Urgency;
use wormhole_core::view::sidebar::spoken;
use wormhole_core::view::sidebar::tabs::{Hit, UNGROUPED, search};

use super::facts::RowInfo;
use super::row::{Frame, glyph_for};
use super::switcher_native::{self as native, Catalog, Group, Source, Terminal};
use super::tab_bar::SetView;
use super::{NewTerminal, Sidebar, SidebarEvent};
use crate::select;
use crate::settings::update_settings;
use crate::settings::{UiColors, settings_in};
use crate::style::{self, Edges, TextRole};
use crate::style::{GroupHead, GroupSwitch, PopoverRow, TileFace, popover_row, popover_rule};

/// What a card or row does once the popover has closed.
type Then = Box<dyn Fn(&mut Sidebar, &mut Window, &mut Context<Sidebar>)>;

/// The popover's width.
const POPOVER_W: f32 = 340.0;
/// The gap between two cards, and how far the popover is inset.
const CARD_GAP: f32 = 6.0;

/// The switcher being open: its search field.
pub(super) struct Switcher {
    input: Entity<InputState>,
    _events: Subscription,
    selected: Option<SwitchTarget>,
}

pub(super) struct NativeSwitcher {
    picker: Entity<crate::picker::Picker>,
    source: Arc<Source>,
    catalog: Catalog,
    _subscription: Subscription,
}

/// A terminal's tile as the popover draws it later, without the workspace at hand.
#[derive(Clone)]
struct TileSnap {
    id: i64,
    face: TileFace,
    glyph: style::StatusGlyph,
}

/// A group's card.
struct CardSnap {
    set: usize,
    label: SharedString,
    color: Option<Hsla>,
    wash: Option<Hsla>,
    pinned: bool,
    /// The loudest first.
    tiles: Vec<TileSnap>,
    count: usize,
    alert: Option<Hsla>,
    current: bool,
    spoken: SharedString,
    group: Option<u32>,
    /// The group's menu is opened by the card's name and a right-click.
    /// The card's name as a field while the group is renamed.
    renaming: Option<Entity<InputState>>,
}

/// A line of the search's results.
struct HitSnap {
    target: SwitchTarget,
    label: SharedString,
    note: SharedString,
    color: Option<Hsla>,
    tile: Option<TileSnap>,
}

/// Stable identities survive a reorder between drawing a result and clicking it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwitchTarget {
    Group(Option<u32>),
    Terminal(i64),
}

fn target_for(hit: Hit, view: &SetView, infos: &[RowInfo]) -> Option<SwitchTarget> {
    match hit {
        Hit::Set(at) => view.looks.get(at).map(|look| SwitchTarget::Group(look.group)),
        Hit::Terminal(_, ix) => infos.get(ix).map(|row| SwitchTarget::Terminal(row.id)),
    }
}

fn group_index(view: &SetView, group: Option<u32>) -> Option<usize> {
    view.looks.iter().position(|look| look.group == group)
}

/// The first result is highlighted until the person chooses another. A vanished target is never
/// replaced by a different result merely because it occupied the same index.
fn selected_target(targets: &[SwitchTarget], selected: Option<SwitchTarget>) -> Option<SwitchTarget> {
    selected.filter(|target| targets.contains(target)).or_else(|| targets.first().copied())
}

fn stepped_target(
    targets: &[SwitchTarget],
    selected: Option<SwitchTarget>,
    forward: bool,
) -> Option<SwitchTarget> {
    let current = selected_target(targets, selected)?;
    let at = targets.iter().position(|target| *target == current)?;
    let next = if forward { (at + 1) % targets.len() } else { (at + targets.len() - 1) % targets.len() };
    targets.get(next).copied()
}

/// Everything the popover draws, copied when the bar is drawn: it builds its content later.
struct Snapshot {
    ui: UiColors,
    bg: Hsla,
    metrics: style::Metrics,
    open: bool,
    input: Option<Entity<InputState>>,
    cards: Vec<CardSnap>,
    /// "Ungrouped": its terminals' number, when there are some.
    loose: Option<(usize, bool)>,
    /// The matches, while something is typed.
    hits: Option<Vec<HitSnap>>,
    selected: Option<SwitchTarget>,
    hint: SharedString,
    keys: SharedString,
}

impl Sidebar {
    /// The switcher in the bar: the current group's dot, name and count with the popover on it.
    pub(super) fn render_switcher(
        &self,
        view: &SetView,
        infos: &[RowInfo],
        frame: &Frame,
        alert: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let current = view.current.unwrap_or(0);
        let (name, count) = view
            .sets
            .get(current)
            .map(|set| (set.label.clone(), set.members.len()))
            .unwrap_or_else(|| (UNGROUPED.to_string(), 0));
        let trigger = style::group_switcher(
            "group-switcher",
            ui,
            GroupSwitch {
                name: name.clone().into(),
                alert,
                spoken: spoken::switcher_label(&name, count, alert).into(),
            },
        );
        let snapshot = self.switcher_snapshot(view, infos, frame, cx);
        let this = cx.weak_entity();
        let on_open = this.clone();
        let popover = Popover::new("group-switcher-popover")
            .anchor(Anchor::TopLeft)
            .offset(px(4.0))
            .open(snapshot.open)
            .on_open_change(move |open, window, cx| {
                on_open
                    .update(cx, |sidebar, cx| {
                        if *open {
                            if !sidebar.open_native_switcher(window, cx) {
                                sidebar.start_switcher(window, cx);
                            }
                        } else {
                            sidebar.finish_switcher(window, cx);
                        }
                    })
                    .ok();
            })
            .trigger(trigger);
        let (ui, bg) = (snapshot.ui, snapshot.bg);
        let popover = select::menu_surface(popover, ui, px(POPOVER_W), bg)
            .p_2()
            .content(move |_, _, cx| switcher_content(&snapshot, this.clone(), cx));
        // The bar's drag takes a press unless the element over it holds the pointer.
        div()
            .id("group-switcher-slot")
            .relative()
            .occlude()
            .flex_none()
            .child(popover)
            .child(self.tabs.switcher_anchor.probe())
            .into_any_element()
    }

    fn native_switcher_catalog(&self, cx: &App) -> Catalog {
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        Catalog {
            show_status: settings_in(&self.input.profile, cx).sidebar_status_dot,
            groups: view
                .sets
                .iter()
                .enumerate()
                .map(|(at, set)| Group {
                    id: view.looks[at].group,
                    label: set.label.clone(),
                    members: set.members.clone(),
                    current: view.current == Some(at),
                    color: view.looks[at].color.map(|color| {
                        let rgb = Rgba::from(color);
                        [rgb.r, rgb.g, rgb.b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
                    }),
                    alerts: set
                        .members
                        .iter()
                        .filter(|&&ix| Urgency::from(glyph_for(&self.input.rows[ix])).is_alert())
                        .count(),
                })
                .collect(),
            terminals: self
                .input
                .rows
                .iter()
                .map(|row| {
                    let glyph = glyph_for(row);
                    let urgency = if row.bell
                        && matches!(
                            glyph,
                            style::StatusGlyph::Shell
                                | style::StatusGlyph::Idle
                                | style::StatusGlyph::Terminal { unread: false }
                        ) {
                        Urgency::Bell
                    } else {
                        Urgency::from(glyph)
                    };
                    Terminal { id: row.id, name: row.name.to_string(), urgency }
                })
                .collect(),
        }
    }

    pub(super) fn refresh_native_switcher(&mut self, cx: &mut Context<Self>) {
        if self.tabs.native_switcher.is_none() {
            return;
        }
        let catalog = self.native_switcher_catalog(cx);
        if !self.input.tabs_on || catalog.groups.len() < 2 {
            self.close_native_switcher(cx);
            return;
        }
        if let Some(native) = self.tabs.native_switcher.as_mut()
            && native.catalog != catalog
        {
            native.source.replace(catalog.clone());
            native.catalog = catalog;
            native.picker.update(cx, |picker, cx| picker.refresh(cx));
        }
    }

    fn close_native_switcher(&mut self, cx: &mut Context<Self>) {
        if let Some(native) = self.tabs.native_switcher.take() {
            native.picker.update(cx, |picker, cx| picker.close(cx));
            cx.notify();
        }
    }

    fn open_native_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        use crate::picker::{Picker, PickerEvent, Sources, Words};
        if self.tabs.native_switcher.is_some() {
            self.close_native_switcher(cx);
            return true;
        }
        let catalog = self.native_switcher_catalog(cx);
        let source = Arc::new(Source::new(catalog.clone()));
        let anchor = self.tabs.switcher_anchor.clone();
        let place: crate::picker::Anchor = Rc::new(move |window, _| {
            let viewport = window.viewport_size();
            let bounds = anchor.bounds();
            (
                Rect::new(0.0, 0.0, f32::from(viewport.width), f32::from(viewport.height)),
                Rect::new(
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ),
            )
        });
        let words = Words {
            placeholder: "Go to group or terminal…".into(),
            name: "Groups and terminals".into(),
            hint: "Right-click a group for its options".into(),
            note: |note| if matches!(note, Note::Empty) { "No matches".into() } else { String::new() },
        };
        let profile = self.input.profile.clone();
        let picker = cx.new(|cx| {
            Picker::new(
                PickerContext::default(),
                Sources { providers: vec![source.clone()], decorators: vec![] },
                words,
                profile,
                place,
                window,
                cx,
            )
        });
        if !picker.read(cx).has_panel() {
            picker.update(cx, |picker, cx| picker.close(cx));
            return false;
        }
        let subscription =
            cx.subscribe_in(&picker, window, |this, _, event: &PickerEvent, window, cx| match event {
                PickerEvent::Insert(key) => {
                    let action = native::action(key);
                    this.close_native_switcher(cx);
                    cx.emit(SidebarEvent::FocusTerminal);
                    match action {
                        Some(native::Action::Open(target)) => this.open_switch_target(target, cx),
                        Some(native::Action::NewGroup) => {
                            cx.emit(SidebarEvent::NewTerminal(NewTerminal::InNewGroup))
                        }
                        Some(native::Action::ShowAll) => update_settings(cx, |s| {
                            s.terminals_in = wormhole_core::view::sidebar::TerminalsIn::Sidebar
                        }),
                        None => {}
                    }
                }
                PickerEvent::Context(key) => {
                    if let Some(native::Action::Open(SwitchTarget::Group(Some(group)))) = native::action(key)
                        && this.view.groups.get(group).is_some()
                    {
                        this.close_native_switcher(cx);
                        this.open_group_popover(group, this.tabs.switcher_anchor.bounds(), window, cx);
                    }
                }
                PickerEvent::Close => {
                    this.close_native_switcher(cx);
                    cx.emit(SidebarEvent::FocusTerminal);
                }
                _ => {}
            });
        self.tabs.native_switcher =
            Some(NativeSwitcher { picker, source, catalog, _subscription: subscription });
        true
    }

    /// The search field opens with the popover.
    pub(super) fn start_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Go to group or terminal…"));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.switcher_enter(window, cx),
            InputEvent::Change => {
                if let Some(switcher) = this.tabs.switcher.as_mut() {
                    switcher.selected = None;
                }
                cx.notify();
            }
            _ => {}
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.tabs.switcher = Some(Switcher { input, _events: events, selected: None });
        cx.notify();
    }

    /// Close the popover and give the keyboard back to the terminal.
    pub(crate) fn finish_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.end_switcher(true, window, cx);
    }

    /// [`Self::finish_switcher`], with the keyboard handed back to the terminal only when `refocus`.
    pub(super) fn end_switcher(&mut self, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.switcher.take().is_some() {
            // A group being renamed on a card goes with it.
            self.end_group_rename(true, refocus, window, cx);
            if refocus {
                cx.emit(SidebarEvent::FocusTerminal);
            }
            cx.notify();
        }
    }

    /// ↩ in the search: the highlighted match, or the first one.
    fn switcher_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(query) = self.tabs.switcher.as_ref().map(|s| s.input.read(cx).value().to_string()) else {
            return;
        };
        let input = self.input.clone();
        let infos = &input.rows;
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        let targets = search(&view.sets, &query, |ix| infos[ix].name.to_string())
            .into_iter()
            .filter_map(|hit| target_for(hit, &view, infos))
            .collect::<Vec<_>>();
        let target = selected_target(&targets, self.tabs.switcher.as_ref().and_then(|s| s.selected));
        self.finish_switcher(window, cx);
        if let Some(target) = target {
            self.open_switch_target(target, cx);
        }
    }

    /// Move through the live results without retaining a position that may now name another item.
    fn switcher_step(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let Some(switcher) = self.tabs.switcher.as_ref() else { return false };
        let query = switcher.input.read(cx).value().to_string();
        if query.trim().is_empty() {
            return false;
        }
        let input = self.input.clone();
        let listing = self.listing(cx);
        let view = self.set_view(&listing, cx);
        let targets = search(&view.sets, &query, |ix| input.rows[ix].name.to_string())
            .into_iter()
            .filter_map(|hit| target_for(hit, &view, &input.rows))
            .collect::<Vec<_>>();
        let selected = stepped_target(&targets, switcher.selected, forward);
        if let Some(switcher) = self.tabs.switcher.as_mut() {
            switcher.selected = selected;
        }
        cx.notify();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_switcher_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(switcher) = self.tabs.switcher.as_ref() {
            switcher.input.update(cx, |input, cx| input.set_value(query.to_owned(), window, cx));
        }
    }

    pub(crate) fn open_switch_target(&mut self, target: SwitchTarget, cx: &mut Context<Self>) {
        match target {
            SwitchTarget::Terminal(id) => {
                if self.input.find(id).is_some() {
                    cx.emit(SidebarEvent::Activate { id, then_focus_list: false });
                }
            }
            SwitchTarget::Group(group) => {
                let listing = self.listing(cx);
                let view = self.set_view(&listing, cx);
                if let Some(at) = group_index(&view, group) {
                    self.open_set(&view.sets[at], group, cx);
                }
            }
        }
    }

    /// New group: a new terminal in a group of its own.
    fn new_group_from_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_switcher(window, cx);
        cx.emit(SidebarEvent::NewTerminal(NewTerminal::InNewGroup));
    }

    fn switcher_snapshot(&self, view: &SetView, infos: &[RowInfo], frame: &Frame, cx: &App) -> Snapshot {
        let ui = frame.ui;
        let urgency = |ix: usize| Urgency::from(glyph_for(&infos[ix]));
        let snap_tile = |ix: usize| {
            let (face, glyph) = super::row::tile_parts(&infos[ix], frame);
            TileSnap { id: infos[ix].id, face, glyph }
        };
        let current = view.current;
        let mut cards = Vec::new();
        let mut loose = None;
        for (at, set) in view.sets.iter().enumerate() {
            let look = &view.looks[at];
            if set.band.is_none() {
                loose = Some((set.members.len(), current == Some(at)));
                continue;
            }
            let alerts = set.members.iter().filter(|&&ix| urgency(ix).is_alert()).count();
            let alert = match set.members.iter().map(|&ix| urgency(ix)).min() {
                Some(Urgency::NeedsYou) => Some(ui.agent_attention),
                Some(Urgency::Failed) => Some(ui.danger),
                _ => None,
            };
            let mut members = set.members.clone();
            members.sort_by_key(|&ix| urgency(ix));
            cards.push(CardSnap {
                set: at,
                label: set.label.clone().into(),
                color: look.color,
                wash: look.wash,
                pinned: look.pinned,
                tiles: members.into_iter().map(snap_tile).collect(),
                count: set.members.len(),
                alert,
                current: current == Some(at),
                spoken: spoken::switcher_card_label(
                    &set.label,
                    set.members.len(),
                    alerts,
                    current == Some(at),
                    at + 1,
                    view.sets.len(),
                )
                .into(),
                group: look.group,
                renaming: look.group.and_then(|group| {
                    let renaming = self.group_renaming.as_ref()?;
                    (renaming.group == group).then(|| renaming.input.clone())
                }),
            });
        }
        let input = self.tabs.switcher.as_ref().map(|s| s.input.clone());
        let query = input.as_ref().map(|input| input.read(cx).value().to_string()).unwrap_or_default();
        let hits = (!query.trim().is_empty()).then(|| {
            search(&view.sets, &query, |ix| infos[ix].name.to_string())
                .into_iter()
                .filter_map(|hit| {
                    let target = target_for(hit, view, infos)?;
                    Some(match hit {
                        Hit::Set(at) => HitSnap {
                            target,
                            label: view.sets[at].label.clone().into(),
                            note: wormhole_core::view::sidebar::wording::count(
                                view.sets[at].members.len(),
                                "terminal",
                                "terminals",
                            )
                            .into(),
                            color: view.looks[at].color,
                            tile: None,
                        },
                        Hit::Terminal(at, ix) => HitSnap {
                            target,
                            label: infos[ix].name.clone(),
                            note: view.sets[at].label.clone().into(),
                            color: None,
                            tile: Some(snap_tile(ix)),
                        },
                    })
                })
                .collect()
        });
        let glyphs = |id: ActionId| keymap::glyphs(id);
        let (previous, next) = (glyphs(ActionId::PreviousGroup), glyphs(ActionId::NextGroup));
        let next_key = next.chars().last().map(String::from).unwrap_or_default();
        Snapshot {
            ui,
            bg: select::menu_bg(ui, cx),
            metrics: style::Metrics {
                card_margin: 0.0,
                card_gap: CARD_GAP,
                ..style::Metrics::of(settings_in(&self.input.profile, cx).density, true)
                    .fading(settings_in(&self.input.profile, cx).fade_group_colors)
                    .with_group_folders(settings_in(&self.input.profile, cx).show_group_folders)
            },
            open: self.tabs.switcher.is_some(),
            input,
            cards,
            loose,
            hits,
            selected: self.tabs.switcher.as_ref().and_then(|s| s.selected),
            hint: format!("{previous} / {next_key} previous, next group").into(),
            keys: format!("{}–9", glyphs(ActionId::Group(1))).into(),
        }
    }
}

/// The popover's content.
fn switcher_content(snap: &Snapshot, this: WeakEntity<Sidebar>, cx: &mut App) -> AnyElement {
    let Snapshot { ui, metrics, input, cards, loose, hits, selected, hint, keys, .. } = snap;
    let (ui, metrics) = (*ui, *metrics);
    // A click that closes the popover first, then does `then`.
    let go = |then: Then| {
        let this = this.clone();
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            this.update(cx, |sidebar, cx| {
                sidebar.finish_switcher(window, cx);
                then(sidebar, window, cx);
            })
            .ok();
        }
    };
    let tile = |tile: &TileSnap, size: f32, cx: &App| {
        let id = ElementId::Name(SharedString::from(format!("switch-tile-{}-{}", tile.id, size as u32)));
        style::status_tile_at(
            id,
            ui,
            style::StatusTile { face: tile.face.clone(), glyph: tile.glyph, selected: false, mini: false },
            size,
            cx,
        )
    };
    let keyboard = this.clone();
    let mut content = div()
        .flex()
        .flex_col()
        .gap(px(CARD_GAP))
        .capture_key_down(move |event: &KeyDownEvent, _, cx| {
            let forward = match event.keystroke.key.as_str() {
                "down" => true,
                "up" => false,
                _ => return,
            };
            if event.keystroke.modifiers != Modifiers::default() {
                return;
            }
            if keyboard.update(cx, |sidebar, cx| sidebar.switcher_step(forward, cx)).unwrap_or(false) {
                cx.stop_propagation();
            }
        })
        .child(
            div().mb_1().child(select::search_row(
                ui,
                true,
                input
                    .as_ref()
                    .map(|input| Input::new(input).appearance(false))
                    .map_or_else(|| div().into_any_element(), IntoElement::into_any_element),
            )),
        );
    if let Some(hits) = hits {
        let mut list = div().flex().flex_col();
        if hits.is_empty() {
            list = list.child(
                div()
                    .px_2()
                    .h(px(style::ROW_H))
                    .flex()
                    .items_center()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("No matches"),
            );
        }
        let active = selected_target(&hits.iter().map(|hit| hit.target).collect::<Vec<_>>(), *selected);
        for (at, hit) in hits.iter().enumerate() {
            let lead = match (&hit.tile, hit.color) {
                (Some(t), _) => tile(t, style::TILE_SMALL, cx),
                (None, color) => {
                    style::color_dot(ui, color.unwrap_or(ui.faint), style::COLOR_DOT_MENU).into_any_element()
                }
            };
            let row = PopoverRow {
                lead: Some(lead),
                note: Some(hit.note.clone()),
                ..PopoverRow::new(hit.label.clone())
            };
            let target = hit.target;
            list = list.child(
                popover_row(("switch-hit", at), ui, row)
                    .aria_selected(Some(hit.target) == active)
                    .when(Some(hit.target) == active, |row| row.bg(ui.hover).aria_active_descendant())
                    .on_click(go(Box::new(move |sidebar, _, cx| sidebar.open_switch_target(target, cx)))),
            );
        }
        return content.child(list).into_any_element();
    }
    for card in cards {
        let set = card.set;
        let target = SwitchTarget::Group(card.group);
        let group = card.group;
        let empty = card.count == 0;
        let (add_this, add_label) = (this.clone(), card.label.clone());
        let label_at = crate::platform::Anchor::default();
        let parts = GroupHead {
            metrics,
            name: card.label.clone(),
            tooltip: None,
            folder: None,
            color: card.color,
            folded: true,
            pinned: card.pinned,
            tiles: card.tiles.iter().map(|t| tile(t, metrics.folded_tile, cx)).collect(),
            count: card.count,
            summary_fade: None,
            alert: card.alert,
            empty: (empty && card.pinned).then(|| {
                let tooltip = SharedString::from(spoken::new_in_group_label(&add_label));
                let add: style::ClickHandler = Box::new(move |_, window, cx| {
                    add_this
                        .update(cx, |sidebar, cx| {
                            sidebar.finish_switcher(window, cx);
                            if let Some(group) = group {
                                cx.emit(SidebarEvent::NewTerminal(NewTerminal::InGroup(group)));
                            }
                        })
                        .ok();
                });
                (tooltip, add)
            }),
            add: None,
            on_toggle: Box::new(go(Box::new(move |sidebar, _, cx| sidebar.open_switch_target(target, cx)))),
            spoken_edit: card.label.clone(),
            spoken_fold: card.spoken.clone(),
            // The name opens the sidebar's group popover, under the name.
            on_label: group.map(|group| {
                let (this, at) = (this.clone(), label_at.clone());
                Box::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    let bounds = at.bounds();
                    this.update(cx, |sidebar, cx| sidebar.open_group_popover(group, bounds, window, cx)).ok();
                }) as style::ClickHandler
            }),
            label_anchor: group.map(|_| label_at.clone()),
            name_edit: card
                .renaming
                .as_ref()
                .map(|input| super::rename::group_name_field(input, this.clone())),
        };
        let head = style::group_head(("switch-card", set), ui, parts)
            .role(Role::Button)
            .aria_label(card.spoken.clone())
            .when_some(group, |head, group| {
                let this = this.clone();
                head.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    cx.stop_propagation();
                    this.update(cx, |sidebar, cx| sidebar.open_group_menu(group, event.position, window, cx))
                        .ok();
                })
            });
        let head = if empty && card.pinned {
            head.on_click(go(Box::new(move |sidebar, _, cx| sidebar.open_switch_target(target, cx))))
        } else {
            head
        };
        let wash = card.wash;
        let segment = style::card_segment(ui, metrics, true, true, wash, head);
        // The open group wears a ring in the primary.
        let ring = card.current.then(|| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .rounded(px(metrics.card_radius))
                .outlined(ui, ui.primary.opacity(RING))
        });
        content = content.child(div().relative().children(Some(segment)).children(ring));
    }
    if let Some((count, current)) = loose {
        let row = PopoverRow {
            note: Some(count.to_string().into()),
            current: *current,
            spoken: spoken::switcher_card_label(UNGROUPED, *count, 0, *current, 0, 0).into(),
            ..PopoverRow::new(UNGROUPED)
        };
        content =
            content.child(popover_row("switch-loose", ui, row).on_click(go(Box::new(|sidebar, _, cx| {
                sidebar.open_switch_target(SwitchTarget::Group(None), cx);
            }))));
    }
    content
        .child(popover_rule(ui))
        .child(
            popover_row(
                "switch-new-group",
                ui,
                PopoverRow {
                    lead: Some(style::icon(IconName::Plus).text_color(ui.muted).into_any_element()),
                    ..PopoverRow::new("New group")
                },
            )
            .on_click({
                let this = this.clone();
                move |_, window, cx| {
                    this.update(cx, |sidebar, cx| sidebar.new_group_from_switcher(window, cx)).ok();
                }
            }),
        )
        .child(
            popover_row("switch-show-all", ui, PopoverRow::new("Show all groups in the sidebar")).on_click({
                let this = this.clone();
                move |_, window, cx| {
                    this.update(cx, |sidebar, cx| sidebar.finish_switcher(window, cx)).ok();
                    update_settings(cx, |s| {
                        s.terminals_in = wormhole_core::view::sidebar::TerminalsIn::Sidebar
                    });
                }
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .pt_1()
                .text_caption()
                .text_color(ui.muted)
                .child(hint.clone())
                .child(keys.clone()),
        )
        .into_any_element()
}

/// How strong the open group's ring is: the primary at this share.
const RING: f32 = 0.6;

#[cfg(test)]
mod tests {
    use super::{
        Hit, RowInfo, SetView, SwitchTarget, group_index, selected_target, stepped_target, target_for,
    };
    use crate::sidebar::tab_bar::SetLook;
    use wormhole_core::view::sidebar::tabs::TabSet;

    fn view(groups: &[Option<u32>]) -> SetView {
        SetView {
            sets: groups
                .iter()
                .map(|_| TabSet { band: None, label: String::new(), members: vec![] })
                .collect(),
            looks: groups
                .iter()
                .map(|group| SetLook { group: *group, color: None, wash: None, pinned: false })
                .collect(),
            current: None,
        }
    }

    #[test]
    fn search_selection_follows_identity_and_wraps_at_the_ends() {
        let a = SwitchTarget::Group(Some(1));
        let b = SwitchTarget::Terminal(2);
        let c = SwitchTarget::Terminal(3);
        let rows = [a, b, c];
        assert_eq!(selected_target(&rows, None), Some(a));
        assert_eq!(stepped_target(&rows, None, true), Some(b));
        assert_eq!(stepped_target(&rows, Some(a), false), Some(c));
        assert_eq!(stepped_target(&rows, Some(c), true), Some(a));
        assert_eq!(selected_target(&[c, b, a], Some(b)), Some(b));
        assert_eq!(selected_target(&[c, a], Some(b)), Some(c));
        assert_eq!(stepped_target(&[], Some(b), true), None);
    }

    #[test]
    fn results_keep_their_identity_when_groups_and_terminals_move_or_disappear() {
        let before = view(&[Some(11), Some(22), None]);
        let rows = [RowInfo::test_row(101), RowInfo::test_row(202)];
        let group = target_for(Hit::Set(1), &before, &rows).expect("group");
        let terminal = target_for(Hit::Terminal(1, 0), &before, &rows).expect("terminal");
        assert_eq!(group, SwitchTarget::Group(Some(22)));
        assert_eq!(terminal, SwitchTarget::Terminal(101));

        let after = view(&[Some(22), Some(11), None]);
        let SwitchTarget::Group(id) = group else { panic!("group") };
        assert_eq!(group_index(&after, id), Some(0));
        assert_eq!(group_index(&view(&[Some(11), None]), id), None);
        let input = crate::sidebar::SidebarInput {
            rows: vec![rows[1].clone(), rows[0].clone()],
            ..Default::default()
        };
        let SwitchTarget::Terminal(id) = terminal else { panic!("terminal") };
        assert_eq!(input.find(id), Some(1));
        assert_eq!(crate::sidebar::SidebarInput::default().find(id), None);
        assert_eq!(target_for(Hit::Set(99), &before, &rows), None);
        assert_eq!(target_for(Hit::Terminal(0, 99), &before, &rows), None);
    }
}
