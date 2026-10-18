//! The tab bar's group switcher and the popover it opens (RF-02 b): a search field ("Go to group or
//! terminal…"), the groups drawn as the sidebar's folded cards (their tiles and count, the open one
//! ringed, a pinned empty one saying "No terminals"), "In no group" with its count, New group and
//! "Show all groups in the sidebar", and the keys' hint. A click on a card goes to the group; typing
//! lists the groups and terminals that match, and ↩ goes to the first.
use gpui_kit::component::input::Input;
use gpui_kit::component::popover::Popover;
use wormhole_core::sidebar::Urgency;
use wormhole_core::sidebar::spoken;
use wormhole_core::sidebar::tabs::{Hit, IN_NO_GROUP, search};

use super::super::settings_page::update_settings;
use super::groups::{PopoverSnapshot, group_popover};
use super::rows::{Frame, glyph_for};
use super::tab_bar::SetView;
use super::*;
use crate::select;
use crate::style::{GroupHead, GroupSwitch, PopoverRow, TileFace, popover_row, popover_rule};

/// What a card or row does once the popover has closed.
type Then = Box<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>;

/// The popover's width.
const POPOVER_W: f32 = 340.0;
/// The gap between two cards, and how far the popover is inset.
const CARD_GAP: f32 = 6.0;

/// The switcher being open: its search field.
pub(in crate::workspace) struct Switcher {
    input: Entity<InputState>,
    _events: Subscription,
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
    pinned: bool,
    /// The loudest first.
    tiles: Vec<TileSnap>,
    count: usize,
    alert: Option<Hsla>,
    current: bool,
    spoken: SharedString,
    group: Option<u32>,
    /// The sidebar label's popover, which the card's name and a right-click open.
    popover: Option<PopoverSnapshot>,
}

/// A line of the search's results.
struct HitSnap {
    hit: Hit,
    label: SharedString,
    note: SharedString,
    color: Option<Hsla>,
    tile: Option<TileSnap>,
}

/// Everything the popover draws, copied when the bar is drawn: it builds its content later.
struct Snapshot {
    ui: UiColors,
    bg: Hsla,
    metrics: style::Metrics,
    open: bool,
    input: Option<Entity<InputState>>,
    cards: Vec<CardSnap>,
    /// "In no group": its terminals' number, when there are some.
    loose: Option<(usize, bool)>,
    /// The matches, while something is typed.
    hits: Option<Vec<HitSnap>>,
    hint: SharedString,
    keys: SharedString,
}

impl Workspace {
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
            .unwrap_or_else(|| (IN_NO_GROUP.to_string(), 0));
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
                    .update(cx, |ws, cx| {
                        if *open {
                            ws.start_switcher(window, cx);
                        } else {
                            ws.finish_switcher(window, cx);
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
        div().id("group-switcher-slot").occlude().flex_none().child(popover).into_any_element()
    }

    /// The search field opens with the popover.
    fn start_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Go to group or terminal…"));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.switcher_enter(window, cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.sidebar.tabs.switcher = Some(Switcher { input, _events: events });
        cx.notify();
    }

    /// Close the popover and give the keyboard back to the terminal.
    pub(super) fn finish_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar.tabs.switcher.take().is_some() {
            // A group popover open on a card goes with it.
            self.finish_group_edit(window, cx);
            window.focus(&self.active_terminal().focus_handle(cx), cx);
            cx.notify();
        }
    }

    /// ↩ in the search: the first match.
    fn switcher_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(query) = self.sidebar.tabs.switcher.as_ref().map(|s| s.input.read(cx).value().to_string())
        else {
            return;
        };
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        let view = self.set_view(&listing, cx);
        let first = search(&view.sets, &query, |ix| infos[ix].name.to_string()).first().copied();
        self.finish_switcher(window, cx);
        match first {
            Some(Hit::Set(at)) => self.open_set_at(at, window, cx),
            Some(Hit::Terminal(_, ix)) => self.activate(ix, window, cx),
            None => {}
        }
    }

    /// New group: a new terminal in a group of its own.
    fn new_group_from_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_switcher(window, cx);
        self.new_session(window, cx);
        if let Some(id) = self.sessions.get(self.active).map(|session| session.id) {
            self.group_terminals(&[id], cx);
        }
    }

    fn switcher_snapshot(&self, view: &SetView, infos: &[RowInfo], frame: &Frame, cx: &App) -> Snapshot {
        let ui = frame.ui;
        let urgency = |ix: usize| Urgency::from(glyph_for(&infos[ix]));
        let snap_tile = |ix: usize| {
            let (face, glyph) = super::rows::tile_parts(&infos[ix], frame);
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
                popover: look.group.map(|group| self.popover_snapshot(group, ui, cx)),
            });
        }
        let input = self.sidebar.tabs.switcher.as_ref().map(|s| s.input.clone());
        let query = input.as_ref().map(|input| input.read(cx).value().to_string()).unwrap_or_default();
        let hits = (!query.trim().is_empty()).then(|| {
            search(&view.sets, &query, |ix| infos[ix].name.to_string())
                .into_iter()
                .map(|hit| match hit {
                    Hit::Set(at) => HitSnap {
                        hit,
                        label: view.sets[at].label.clone().into(),
                        note: wormhole_core::sidebar::text::count(
                            view.sets[at].members.len(),
                            "terminal",
                            "terminals",
                        )
                        .into(),
                        color: view.looks[at].color,
                        tile: None,
                    },
                    Hit::Terminal(at, ix) => HitSnap {
                        hit,
                        label: infos[ix].name.clone(),
                        note: view.sets[at].label.clone().into(),
                        color: None,
                        tile: Some(snap_tile(ix)),
                    },
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
                ..style::Metrics::of(settings_in(&self.profile, cx).density, true)
            },
            open: self.sidebar.tabs.switcher.is_some(),
            input,
            cards,
            loose,
            hits,
            hint: format!("{previous} / {next_key} previous, next group").into(),
            keys: format!("{}–9", glyphs(ActionId::Group(1))).into(),
        }
    }
}

/// The popover's content.
fn switcher_content(snap: &Snapshot, this: WeakEntity<Workspace>, cx: &mut App) -> AnyElement {
    let Snapshot { ui, metrics, input, cards, loose, hits, hint, keys, .. } = snap;
    let (ui, metrics) = (*ui, *metrics);
    // A click that closes the popover first, then does `then`.
    let go = |then: Then| {
        let this = this.clone();
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            this.update(cx, |ws, cx| {
                ws.finish_switcher(window, cx);
                then(ws, window, cx);
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
    let mut content = div().flex().flex_col().gap(px(CARD_GAP)).child(
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
            let target = hit.hit;
            list = list.child(popover_row(("switch-hit", at), ui, row).on_click(go(Box::new(
                move |ws, window, cx| match target {
                    Hit::Set(at) => ws.open_set_at(at, window, cx),
                    Hit::Terminal(_, ix) => ws.activate(ix, window, cx),
                },
            ))));
        }
        return content.child(list).into_any_element();
    }
    for card in cards {
        let set = card.set;
        let group = card.group;
        let empty = card.count == 0;
        let (add_this, add_label) = (this.clone(), card.label.clone());
        let parts = GroupHead {
            metrics,
            name: card.label.clone(),
            color: card.color,
            folded: true,
            pinned: card.pinned,
            tiles: card.tiles.iter().map(|t| tile(t, metrics.folded_tile, cx)).collect(),
            count: card.count,
            alert: card.alert,
            empty: (empty && card.pinned).then(|| {
                let tooltip = SharedString::from(spoken::new_in_group_label(&add_label));
                let add: style::ClickHandler = Box::new(move |_, window, cx| {
                    add_this
                        .update(cx, |ws, cx| {
                            ws.finish_switcher(window, cx);
                            if let Some(group) = group {
                                ws.new_session_in_group(group, window, cx);
                            }
                        })
                        .ok();
                });
                (tooltip, add)
            }),
            on_toggle: Box::new(go(Box::new(move |ws, window, cx| ws.open_set_at(set, window, cx)))),
            spoken_edit: card.label.clone(),
            spoken_fold: card.spoken.clone(),
            // The sidebar's popover on the name; one is open at a time (`sidebar.editing`).
            wrap_label: match card.popover.clone() {
                Some(snap) => {
                    let this = this.clone();
                    Box::new(move |trigger| group_popover(snap, trigger, this))
                }
                None => Box::new(|trigger| trigger.into_any_element()),
            },
        };
        let head = style::group_head(("switch-card", set), ui, parts)
            .role(Role::Button)
            .aria_label(card.spoken.clone())
            .when_some(group, |head, group| {
                let this = this.clone();
                head.on_mouse_down(MouseButton::Right, move |_, window, cx| {
                    cx.stop_propagation();
                    this.update(cx, |ws, cx| ws.start_group_edit(group, window, cx)).ok();
                })
            });
        let head = if empty && card.pinned {
            head.on_click(go(Box::new(move |ws, window, cx| ws.open_set_at(set, window, cx))))
        } else {
            head
        };
        let wash = card.color;
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
            spoken: spoken::switcher_card_label(IN_NO_GROUP, *count, 0, *current, 0, 0).into(),
            ..PopoverRow::new(IN_NO_GROUP)
        };
        content =
            content.child(popover_row("switch-loose", ui, row).on_click(go(Box::new(|ws, window, cx| {
                let infos = ws.row_infos(cx);
                let listing = ws.listing(&infos, cx);
                let view = ws.set_view(&listing, cx);
                if let Some(at) = view.sets.iter().position(|set| set.band.is_none()) {
                    ws.open_set_at(at, window, cx);
                }
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
                    this.update(cx, |ws, cx| ws.new_group_from_switcher(window, cx)).ok();
                }
            }),
        )
        .child(
            popover_row("switch-show-all", ui, PopoverRow::new("Show all groups in the sidebar")).on_click({
                let this = this.clone();
                move |_, window, cx| {
                    this.update(cx, |ws, cx| ws.finish_switcher(window, cx)).ok();
                    update_settings(cx, |s| s.terminals_in = wormhole_core::sidebar::TerminalsIn::Sidebar);
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
