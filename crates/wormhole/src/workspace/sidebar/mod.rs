//! The terminal list on the left. Core
//! (`wormhole_core::sidebar`) decides what is listed, in what order, with which ⌘ numbers
//! and in which words; this gathers each terminal's facts, draws the groups and rows, and
//! wires up the ⌘-hold hints (`hints`), the row and group menus (`row_menu`, `groups`) and drag
//! and drop (`drag`).
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::glass::{GlassKind, Insets, LIST_CLIP, glass_key};
use super::session::home_dir;
use super::terminal_drag::Over;
use super::*;

use wormhole_core::agents::AgentId;
use wormhole_core::sidebar::NUMBERED;
use wormhole_core::sidebar::hints::CmdHold;
use wormhole_core::sidebar::order::FrozenNumbers;
use wormhole_core::sidebar::{
    Card, Entry, ListInput, Listing, RowState, SidebarNumbers, Splits, WindowSidebar,
};

mod demo;
mod drag;
mod facts;
mod groups;
mod hints;
mod keys;
mod motion;
mod row_menu;
mod rows;
mod tab_bar;
mod tick;

use facts::{RowInfo, row_times};
use motion::{ItemKey, ListMotion};
pub(super) use row_menu::reveal_in_finder;

/// The sidebar's part of a window: what is saved with it (the groups, their order, which are
/// folded) and what it is doing right now.
pub(super) struct SidebarState {
    /// Saved in the window's layout.
    pub(super) view: WindowSidebar,
    /// ⌘ held on its own (the ⌘1–9 keycaps), and the timer that shows them.
    hold: CmdHold,
    hold_timer: Option<Task<()>>,
    _keys: Subscription,
    /// The terminal whose context menu is open: its row wears the focus ring.
    menu_row: Option<i64>,
    /// The group whose head's context menu is open.
    menu_group: Option<u32>,
    /// The group whose name is being edited in place.
    renaming: Option<groups::Renaming>,
    menu_dismissed: Option<Subscription>,
    pub(super) scroll: ScrollHandle,
    /// Bring this terminal's row into view on the next frame.
    pub(super) scroll_to: Option<i64>,
    /// The row that just moved to another group, flashed once: its terminal and a counter that
    /// restarts the flash.
    pub(super) flash: Option<(i64, u64)>,
    /// What the pointer is over while something is dragged over the list.
    pub(super) over: Option<Over>,
    /// How many times the person rearranged the list (a drag, a group made or taken apart): the
    /// ⌘ numbers are dealt again when it changes.
    order_rev: u64,
    /// The ⌘ numbers frozen to their terminals (by id) under "Stay with the terminal".
    numbers: RefCell<FrozenNumbers<i64>>,
    /// The list's lines as last drawn and the slide of those that moved.
    motion: ListMotion,
    /// Which repository a worktree belongs to, by its root (read from disk once per refresh).
    worktrees: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
    /// The times the rows showed when last drawn ("Working · 3m", "1:12"), so a tick redraws
    /// only when one of them would read differently.
    times_drawn: RefCell<Vec<Option<String>>>,
    /// The terminal welcome's example group while its sidebar step is up (`demo`). Never saved.
    pub(super) demo: Option<demo::Demo>,
    /// The tab bar, while the terminals are in tabs.
    pub(super) tabs: tab_bar::TabBarState,
}

impl SidebarState {
    pub(super) fn new(view: WindowSidebar, window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        let keys = hints::watch_keys(window, cx);
        Self {
            view,
            hold: CmdHold::default(),
            hold_timer: None,
            _keys: keys,
            menu_row: None,
            menu_group: None,
            renaming: None,
            menu_dismissed: None,
            scroll: ScrollHandle::new(),
            scroll_to: None,
            flash: None,
            over: None,
            order_rev: 0,
            numbers: RefCell::default(),
            motion: ListMotion::default(),
            worktrees: RefCell::default(),
            times_drawn: RefCell::default(),
            demo: None,
            tabs: tab_bar::TabBarState::default(),
        }
    }
}

impl Workspace {
    /// The list as the sidebar draws it.
    fn listing(&self, infos: &[RowInfo], cx: &App) -> Listing {
        let ids: Vec<i64> = infos.iter().map(|info| info.id).collect();
        let pinned: Vec<bool> = infos.iter().map(|info| info.pinned).collect();
        let split_of = self.split_ids();
        let numbers = settings(cx).sidebar_numbers;
        let mut listing = Listing::build(&ListInput {
            ids: &ids,
            pinned: &pinned,
            sidebar: &self.sidebar.view,
            splits: Splits { split_of: &split_of, selected: Some(self.active) },
            numbers,
        });
        self.freeze_numbers(&mut listing, infos, numbers);
        listing
    }

    /// Under "Stay with the terminal", the numbers the listing dealt are replaced by the frozen ones:
    /// kept while the terminals are only added to, dealt again when the person rearranges the list,
    /// folds a group, closes a terminal or changes the setting (`FrozenNumbers`).
    fn freeze_numbers(&self, listing: &mut Listing, infos: &[RowInfo], numbers: SidebarNumbers) {
        let mut frozen = self.sidebar.numbers.borrow_mut();
        if numbers != SidebarNumbers::Terminal {
            frozen.reset();
            return;
        }
        let mut tag = DefaultHasher::new();
        self.sidebar.order_rev.hash(&mut tag);
        self.sidebar.view.folded.hash(&mut tag);
        let window: Vec<(i64, bool)> = infos.iter().map(|info| (info.id, info.pinned)).collect();
        let dealt: Vec<i64> = listing.visible().map(|ix| infos[ix].id).collect();
        let held = frozen.settle(&window, tag.finish(), &dealt, NUMBERED);
        let at = |id: &i64| infos.iter().position(|info| info.id == *id);
        listing.set_numbers(held.iter().filter_map(at).collect());
    }

    /// The listing without drawing it (shortcuts, devctl).
    pub(super) fn session_listing(&self, cx: &App) -> Listing {
        self.listing(&self.row_infos(cx), cx)
    }

    /// Every terminal top to bottom as the sidebar lists it (folded ones included).
    pub(super) fn session_order(&self, cx: &App) -> Vec<usize> {
        self.session_listing(cx).order().to_vec()
    }

    /// The window's terminals' ids, in the order they were made.
    pub(super) fn session_ids(&self) -> Vec<i64> {
        self.sessions.iter().map(|session| session.id).collect()
    }

    /// The person rearranged the list: the ⌘ numbers are dealt again.
    pub(super) fn list_rearranged(&mut self, cx: &mut Context<Self>) {
        self.sidebar.order_rev += 1;
        self.mark_dirty(cx);
    }

    /// The sidebar, `width` points wide (its column's, or its width at rest while it slides).
    pub(super) fn render_sidebar(
        &mut self,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let glass = self.glass_on(cx);
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        *self.sidebar.times_drawn.borrow_mut() = row_times(&infos, cx);
        if let Some(id) = self.sidebar.scroll_to.take()
            && let Some(at) = listing
                .entries
                .iter()
                .position(|e| matches!(e, Entry::Row { session, .. } if infos[*session].id == id))
        {
            self.sidebar.scroll.scroll_to_item(at);
        }
        let keys: Vec<ItemKey> = listing
            .entries
            .iter()
            .map(|entry| match *entry {
                Entry::Band { band, .. } => ItemKey::Band(listing.bands[band].key()),
                Entry::Row { session, .. } => ItemKey::Row(infos[session].id),
            })
            .collect();
        self.plan_list_motion(keys.clone(), cx);
        let frame = rows::Frame::new(self, width, window, cx);
        let mark = self.drop_mark(&listing, &infos);
        let held = cx.has_active_drag().then(|| self.terminal_drag.as_ref().map(|drag| drag.id)).flatten();
        let mut items: Vec<AnyElement> = Vec::with_capacity(listing.entries.len());
        for (at, entry) in listing.entries.iter().enumerate() {
            let line = match *entry {
                Entry::Band { band, position, of } => {
                    self.render_group_band(&listing, band, (position, of), &frame, window, cx)
                }
                Entry::Row { session, band, position, of, number, marks } => {
                    let place = rows::Place { position, of, number, grouped: band.is_some(), marks };
                    self.render_row(&infos[session], &place, &frame, window, cx)
                }
            };
            // Where a dragged thing would land, as a line over this one.
            let line = match self.mark_element(mark.as_ref(), &keys[at], ui) {
                Some(mark) => div().relative().child(line).child(mark).into_any_element(),
                None => line,
            };
            // Each line is a stretch of its card: a group's head and rows, or a terminal's own.
            let (top, bottom) = listing.card_edges(at);
            let segment = style::card_segment(ui, top, bottom, true, false, line);
            // Each card is a glass of its own.
            let segment = if glass {
                let (key, inset) = match listing.card(at) {
                    Some(Card::Group(band)) => (
                        glass_key(GlassKind::Card, 1 + band as u64),
                        Insets(
                            style::CARD_MARGIN,
                            0.0,
                            style::CARD_MARGIN,
                            if bottom { style::CARD_GAP } else { 0.0 },
                        ),
                    ),
                    Some(Card::Loose(session)) => (
                        glass_key(GlassKind::Loose, infos[session].id as u64),
                        Insets(style::CARD_MARGIN, 0.0, style::CARD_MARGIN, style::CARD_GAP),
                    ),
                    None => (0, Insets(0.0, 0.0, 0.0, 0.0)),
                };
                segment.child(self.glass_probe(key, inset, style::CARD_RADIUS, Some(LIST_CLIP)))
            } else {
                segment
            };
            let line = match *entry {
                Entry::Row { session, .. } => {
                    let id = infos[session].id;
                    let group = listing.card(at).and_then(|card| match card {
                        Card::Group(band) => Some(listing.bands[band].group),
                        Card::Loose(_) => None,
                    });
                    self.with_row_drop(segment.id(("row-line", id as u64)), id, group, bottom, cx)
                        .into_any_element()
                }
                Entry::Band { band, .. } => {
                    let group = listing.bands[band].group;
                    self.with_head_drop(segment.id(("head-line", group as u64)), group, bottom, cx)
                        .into_any_element()
                }
            };
            let is_held = matches!(&keys[at], ItemKey::Row(id) if Some(*id) == held);
            items.push(self.with_list_motion(keys[at].clone(), line, is_held));
        }
        // After the person's own terminals: the welcome's example group, which is none of them.
        items.extend(self.render_demo(&frame, cx));

        div()
            .id("sidebar")
            .relative()
            .flex()
            .flex_col()
            .w(px(width))
            // Long, untruncated text must never widen the panel.
            .min_w(px(width))
            .max_w(px(width))
            .overflow_hidden()
            .h_full()
            .flex_none()
            .bg(if glass { gpui_kit::transparent_black() } else { ui.panel })
            .child(self.render_sidebar_header(window, cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(glass, |d| d.child(self.glass_clip_probe(LIST_CLIP)))
                    .child({
                        let list = div()
                            .id("session-list")
                            .role(Role::Tree)
                            .aria_label("Terminals")
                            .size_full()
                            .flex()
                            .flex_col()
                            .pt_1()
                            .overflow_y_scroll()
                            .track_scroll(&self.sidebar.scroll)
                            .children(items);
                        let list = self.with_list_drop(list.child(self.list_tail(cx)), cx);
                        // It holds the keyboard for the sidebar.
                        self.area_container(wormhole_core::focus::Area::Sidebar, list, cx)
                    }),
            )
            .children(self.render_no_agents_note(cx))
            .into_any_element()
    }

    /// Fold the group called `label`, or open it; `false` if there is none.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_fold(&mut self, label: &str, cx: &mut Context<Self>) -> bool {
        let listing = self.session_listing(cx);
        let Some(band) = listing.bands.iter().find(|band| band.label == label) else { return false };
        let group = band.group;
        self.toggle_group_band(group, cx);
        true
    }

    /// A new group named `name` holding the terminals at `rows` (as `state` numbers them, from 0);
    /// its id.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_new_group(&mut self, name: &str, rows: &[usize], cx: &mut Context<Self>) -> u32 {
        let ids: Vec<i64> = rows.iter().filter_map(|&ix| self.sessions.get(ix).map(|s| s.id)).collect();
        let group = self.group_terminals(&ids, cx);
        self.sidebar.view.groups.rename(group, name);
        self.mark_dirty(cx);
        group
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_rename(&mut self, ix: usize, name: &str, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get_mut(ix) {
            session.custom_name = (!name.is_empty()).then(|| name.to_string());
            self.mark_dirty(cx);
        }
    }

    /// Scroll the terminal list by `dy` (negative = up), within its bounds.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_sidebar_scroll(&mut self, dy: f32, cx: &mut Context<Self>) {
        let scroll = &self.sidebar.scroll;
        let (offset, max) = (scroll.offset(), scroll.max_offset());
        let y = (offset.y - px(dy)).clamp(-max.y, px(0.0));
        scroll.set_offset(point(offset.x, y));
        cx.notify();
    }
}

/// A tooltip of two lines: what it is, then where or what else.
fn two_line_tooltip(
    first: SharedString,
    second: SharedString,
    ui: UiColors,
    window: &mut Window,
    cx: &mut App,
) -> AnyView {
    gpui_kit::component::tooltip::Tooltip::element(move |_, _| {
        div()
            .flex()
            .flex_col()
            .child(div().text_body().font_weight(FontWeight::MEDIUM).child(first.clone()))
            .when(!second.is_empty(), |d| {
                d.child(div().text_caption().text_color(ui.muted).child(second.clone()))
            })
    })
    .build(window, cx)
}

/// The type roles' sizes in rems (`text_body`, `text_label`, `text_caption`), for measuring
/// whether a name is cut off.
const BODY_REM: f32 = 0.875;
const CAPTION_REM: f32 = 0.75;
