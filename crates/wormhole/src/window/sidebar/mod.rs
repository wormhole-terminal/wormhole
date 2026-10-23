//! The terminal list on the left, an entity of its own (`Sidebar`) that the window hosts. Core
//! (`wormhole_core::view::sidebar`) decides what is listed, in what order, with which ⌘ numbers
//! and in which words; this holds the list's state, draws the groups and rows, and wires up the
//! ⌘-hold hints (`hints`), the row and group menus (`row_menu`, `groups`) and drag and drop (`drag`).
//!
//! The sidebar never reads the `Workspace`. The window pushes it a [`SidebarInput`] (the terminals as
//! rows, which is selected, the profile, the width, the glass, the drag in flight) before it is drawn
//! and before the window asks it for something that depends on the terminals as they are; what the
//! person does in the list comes back as a [`SidebarEvent`] the window handles (`window::sidebar_host`).
//! The window reaches the list's state through the methods marked `pub(in crate::window)`.
use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::focus::list::TypeAhead;
use wormhole_core::view::sidebar::NUMBERED;
use wormhole_core::view::sidebar::hints::CmdHold;
use wormhole_core::view::sidebar::order::FrozenNumbers;
use wormhole_core::view::sidebar::{Card, Entry, ListInput, Listing, SidebarNumbers, Splits, WindowSidebar};

use crate::settings::{UiColors, settings_in, store_in, ui_in};
use crate::style::{self, TextRole};
use crate::window::focus::SidebarStop;
use crate::window::glass::{GlassKind, Insets, LIST_CLIP, clip_probe, glass_key, probe};
use crate::window::terminal_drag::Over;

mod demo;
mod drag;
mod facts;
mod group_head;
mod group_popover;
mod groups;
mod hints;
mod input;
mod keys;
mod motion;
pub(in crate::window) mod profile_chip;
mod rename;
mod row_menu;
mod rows;
pub(crate) mod shells;
#[cfg(test)]
mod sidebar_tests;
mod switcher;
mod tab_bar;

pub(in crate::window) use facts::row_times;
pub(in crate::window) use facts::{RowExtra, RowExtras, RowFailure, RowInfo};
pub(in crate::window) use input::{NewTerminal, SidebarEvent, SidebarInput};
use motion::{ItemKey, ListMotion};
pub(in crate::window) use row_menu::reveal_in_finder;
pub(in crate::window) use tab_bar::TAB_ROW_H;

/// The terminal list: what is saved with the window (the groups, their order, which are folded) and what
/// it is doing right now.
pub(in crate::window) struct Sidebar {
    /// Saved in the window's layout.
    view: WindowSidebar,
    /// What the window last said of itself.
    input: Rc<SidebarInput>,
    /// ⌘ held on its own (the ⌘1–9 keycaps), and the timer that shows them.
    hold: CmdHold,
    hold_timer: Option<Task<()>>,
    _keys: Subscription,
    /// The list's focus: the keys walk it while it has the keyboard (F6, ⇥).
    focus: FocusHandle,
    /// The item the keys are on.
    keys_at: Option<SidebarStop>,
    /// Letters typed in the list to jump to a name.
    typed: TypeAhead,
    /// The terminal whose context menu is open: its row wears the focus ring.
    menu_row: Option<i64>,
    /// The group whose head's context menu is open.
    menu_group: Option<u32>,
    /// The group being renamed on its head.
    group_renaming: Option<rename::GroupRename>,
    /// The terminal being renamed in its row.
    renaming: Option<rename::TerminalRename>,
    scroll: ScrollHandle,
    /// The row that just moved to another group, flashed once: its terminal and a counter that
    /// restarts the flash.
    flash: Option<(i64, u64)>,
    /// What the pointer is over while something is dragged over the list.
    over: Option<Over>,
    /// How many times the person rearranged the list (a drag, a group made or taken apart): the
    /// ⌘ numbers are dealt again when it changes.
    order_rev: u64,
    /// The ⌘ numbers frozen to their terminals (by id) under "Stay with the terminal".
    numbers: RefCell<FrozenNumbers<i64>>,
    /// The list's lines as last drawn and the slide of those that moved.
    motion: ListMotion,
    /// The times the rows showed when last drawn ("Working · 3m", "1:12"), so a tick redraws
    /// only when one of them would read differently.
    times_drawn: Vec<Option<String>>,
    /// The terminal welcome's example group while its sidebar step is up (`demo`). Never saved.
    demo: Option<demo::Demo>,
    /// The tab bar, while the terminals are in tabs.
    tabs: tab_bar::TabBarState,
    /// The profile chip's menu being open, and the list it shows.
    profile_menu: Option<profile_chip::ProfilePage>,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    /// A list with `view` saved from the window it was in, whose keys go to `focus` (the area's handle).
    pub(in crate::window) fn new(
        view: WindowSidebar,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let keys = hints::watch_keys(window, cx);
        Self {
            view,
            input: Rc::default(),
            hold: CmdHold::default(),
            hold_timer: None,
            _keys: keys,
            focus,
            keys_at: None,
            typed: TypeAhead::default(),
            menu_row: None,
            menu_group: None,
            group_renaming: None,
            renaming: None,
            scroll: ScrollHandle::new(),
            flash: None,
            over: None,
            order_rev: 0,
            numbers: RefCell::default(),
            motion: ListMotion::default(),
            times_drawn: Vec::new(),
            demo: None,
            tabs: tab_bar::TabBarState::default(),
            profile_menu: None,
        }
    }

    /// What the window says of itself now: what the next frame, and every call that reads the
    /// terminals, goes by.
    pub(in crate::window) fn set_input(&mut self, input: SidebarInput) {
        self.input = Rc::new(input);
    }

    /// The column's width as last said, for a push that changes nothing else.
    pub(in crate::window) fn width(&self) -> f32 {
        self.input.width
    }

    /// The saved view: the groups, their order, which are folded.
    pub(in crate::window) fn view(&self) -> &WindowSidebar {
        &self.view
    }

    /// The saved view, to change (the window seats a joined terminal beside its mates, closes one).
    /// The caller says `Changed` itself when it matters.
    pub(in crate::window) fn view_mut(&mut self) -> &mut WindowSidebar {
        &mut self.view
    }

    /// The list's scroll, which devctl and the tests turn and measure.
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::window) fn scroll(&self) -> &ScrollHandle {
        &self.scroll
    }

    /// What the pointer is over in the list while something is dragged over it.
    #[cfg(test)]
    pub(in crate::window) fn over(&self) -> Option<Over> {
        self.over
    }

    /// The pointer left the list for the panes: nothing in the list is the target. Whether it was.
    pub(in crate::window) fn take_over(&mut self) -> bool {
        self.over.take().is_some()
    }

    /// The list's focus handle, which holds the keyboard while the list is the area the keys are in.
    pub(in crate::window) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The item the keys are on.
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::window) fn keys_at(&self) -> Option<&SidebarStop> {
        self.keys_at.as_ref()
    }

    /// The terminal whose row wears the ring for an open menu.
    #[cfg(test)]
    pub(in crate::window) fn menu_row(&self) -> Option<i64> {
        self.menu_row
    }

    /// The group being renamed, and its name field.
    #[cfg(test)]
    pub(in crate::window) fn group_renaming(
        &self,
    ) -> Option<(u32, Entity<gpui_kit::component::input::InputState>)> {
        self.group_renaming.as_ref().map(|r| (r.group, r.input.clone()))
    }

    /// Whether the terminal welcome's example band is up.
    #[cfg(test)]
    pub(in crate::window) fn has_demo(&self) -> bool {
        self.demo.is_some()
    }

    /// The profile chip's menu is open.
    #[cfg(test)]
    pub(in crate::window) fn profile_menu_open(&self) -> bool {
        self.profile_menu.is_some()
    }

    /// The terminal being renamed, and its name field.
    #[cfg(test)]
    pub(in crate::window) fn terminal_renaming(
        &self,
    ) -> Option<Entity<gpui_kit::component::input::InputState>> {
        self.renaming.as_ref().map(|r| r.input.clone())
    }

    /// Whether the times on the rows read as `now` says: no, and the list wants a redraw (a row's
    /// "Working · 3m" moved on its own).
    pub(in crate::window) fn times_changed(&mut self, now: Vec<Option<String>>) -> bool {
        if self.times_drawn == now {
            return false;
        }
        self.times_drawn = now;
        true
    }

    /// The list as the sidebar draws it, from `input`'s terminals.
    pub(in crate::window) fn listing_of(&self, input: &SidebarInput, cx: &App) -> Listing {
        let ids = input.ids();
        let pinned = input.pinned();
        let split_of = input.split_of();
        let numbers = settings_in(&input.profile, cx).sidebar_numbers;
        let mut listing = Listing::build(&ListInput {
            ids: &ids,
            pinned: &pinned,
            sidebar: &self.view,
            splits: Splits { split_of: &split_of, selected: Some(input.active) },
            numbers,
        });
        self.freeze_numbers(&mut listing, input, numbers);
        listing
    }

    /// The list as the window last described it.
    fn listing(&self, cx: &App) -> Listing {
        self.listing_of(&self.input, cx)
    }

    /// Under "Stay with the terminal", the numbers the listing dealt are replaced by the frozen ones:
    /// kept while the terminals are only added to, dealt again when the person rearranges the list,
    /// folds a group, closes a terminal or changes the setting (`FrozenNumbers`).
    fn freeze_numbers(&self, listing: &mut Listing, input: &SidebarInput, numbers: SidebarNumbers) {
        let mut frozen = self.numbers.borrow_mut();
        if numbers != SidebarNumbers::Terminal {
            frozen.reset();
            return;
        }
        let mut tag = DefaultHasher::new();
        self.order_rev.hash(&mut tag);
        self.view.folded.hash(&mut tag);
        let window: Vec<(i64, bool)> = input.rows.iter().map(|info| (info.id, info.pinned)).collect();
        let dealt: Vec<i64> = listing.visible().map(|ix| input.rows[ix].id).collect();
        let held = frozen.settle(&window, tag.finish(), &dealt, NUMBERED);
        let at = |id: &i64| input.rows.iter().position(|info| info.id == *id);
        listing.set_numbers(held.iter().filter_map(at).collect());
    }

    /// The person rearranged the list: the ⌘ numbers are dealt again, and the window saves.
    pub(in crate::window) fn list_rearranged(&mut self, cx: &mut Context<Self>) {
        self.order_rev += 1;
        self.changed(cx);
    }

    /// The saved view changed: tell the window to save it, and draw.
    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(SidebarEvent::Changed);
        cx.notify();
    }

    /// The list: what lies under the sidebar's header (the window's).
    fn render_list(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = self.input.clone();
        let infos = &input.rows;
        // A drag that ended outside every drop target never heard its drop.
        if !cx.has_active_drag() {
            self.over = None;
        }
        let ui = ui_in(&input.profile, cx);
        let glass = input.glass.clone();
        let listing = self.listing(cx);
        self.times_drawn = row_times(infos, &input.profile, cx);
        let keys: Vec<ItemKey> = listing
            .entries
            .iter()
            .map(|entry| match *entry {
                Entry::Band { band, .. } => ItemKey::Band(listing.bands[band].key()),
                Entry::Row { session, .. } => ItemKey::Row(infos[session].id),
            })
            .collect();
        self.plan_list_motion(keys.clone(), cx);
        let frame = rows::Frame::new(&input, input.width, self.hints_showing(cx), window, cx);
        let mark = self.drop_mark(&listing, infos);
        let group_wash: std::collections::HashMap<u32, Hsla> = {
            let tokens = &store_in(&input.profile, cx).tokens;
            self.view
                .groups
                .iter()
                .filter_map(|group| {
                    let color = group.paint()?;
                    Some((group.id, crate::theme_bridge::hsla(color.wash(tokens))))
                })
                .collect()
        };
        let held = cx.has_active_drag().then(|| input.terminal_drag.map(|(id, _)| id)).flatten();
        let mut items: Vec<AnyElement> = Vec::with_capacity(listing.entries.len());
        // The changes the previous row of the card counted: a group's terminals in one repository
        // say "±3" once, not on every row.
        let mut counted_above: Option<(Option<usize>, &Option<std::path::PathBuf>, usize)> = None;
        for (at, entry) in listing.entries.iter().enumerate() {
            let line = match *entry {
                Entry::Band { band, position, of } => {
                    self.render_group_band(&listing, infos, band, (position, of), &frame, window, cx)
                }
                Entry::Row { session, band, position, of, number, marks } => {
                    let info = &infos[session];
                    // Only inside a group's card: a terminal in no group is a card of its own.
                    let changes_above = band.is_some()
                        && info.repo.is_some()
                        && info.changes.is_some_and(|n| counted_above == Some((band, &info.repo, n)));
                    counted_above = info.changes.map(|n| (band, &info.repo, n));
                    let place = rows::Place {
                        solo: frame.metrics.cards && matches!(listing.card(at), Some(Card::Loose(_))),
                        position,
                        of,
                        number,
                        grouped: band.is_some(),
                        marks,
                        changes_above,
                    };
                    self.render_row(info, &place, &frame, window, cx)
                }
            };
            // Where a dragged thing would land, as a line over this one.
            let line = match self.mark_element(mark.as_ref(), &keys[at], ui) {
                Some(mark) => div().relative().child(line).child(mark).into_any_element(),
                None => line,
            };
            // Each line is a stretch of its card: a group's head and rows, or a terminal's own.
            let (top, bottom) = listing.card_edges(at);
            // A group's colour washes its card from the left.
            let wash = match listing.card(at) {
                Some(Card::Group(band)) => group_wash.get(&listing.bands[band].group).copied(),
                _ => None,
            };
            let segment = match (listing.card(at), entry) {
                (Some(Card::Loose(_)), Entry::Row { session, .. }) => {
                    style::card_segment_solo(ui, frame.metrics, *session == input.active, line)
                }
                _ => style::card_segment(ui, frame.metrics, top, bottom, wash, line),
            };
            // Each card is a glass of its own.
            let segment = match glass.as_ref().filter(|_| frame.metrics.cards) {
                Some(glass) => {
                    let (key, inset) = match listing.card(at) {
                        Some(Card::Group(band)) => (
                            glass_key(GlassKind::Card, 1 + band as u64),
                            Insets(
                                frame.metrics.card_margin,
                                0.0,
                                frame.metrics.card_margin,
                                if bottom { frame.metrics.card_gap } else { 0.0 },
                            ),
                        ),
                        Some(Card::Split(first)) => (
                            glass_key(GlassKind::Loose, infos[first].id as u64),
                            Insets(
                                frame.metrics.card_margin,
                                0.0,
                                frame.metrics.card_margin,
                                if bottom { frame.metrics.card_gap } else { 0.0 },
                            ),
                        ),
                        Some(Card::Loose(session)) => (
                            glass_key(GlassKind::Loose, infos[session].id as u64),
                            Insets(
                                frame.metrics.card_margin,
                                0.0,
                                frame.metrics.card_margin,
                                frame.metrics.card_gap,
                            ),
                        ),
                        None => (0, Insets(0.0, 0.0, 0.0, 0.0)),
                    };
                    segment.child(probe(glass, key, inset, frame.metrics.card_radius, Some(LIST_CLIP)))
                }
                None => segment,
            };
            let line = match *entry {
                Entry::Row { session, .. } => {
                    let id = infos[session].id;
                    let group = listing.card(at).and_then(|card| match card {
                        Card::Group(band) => Some(listing.bands[band].group),
                        Card::Loose(_) | Card::Split(_) => None,
                    });
                    self.with_row_drop(
                        segment.id(("row-line", id as u64)),
                        id,
                        group,
                        bottom.then_some(frame.metrics.card_gap),
                        cx,
                    )
                    .into_any_element()
                }
                Entry::Band { band, .. } => {
                    let group = listing.bands[band].group;
                    self.with_head_drop(
                        segment.id(("head-line", group as u64)),
                        group,
                        bottom.then_some(frame.metrics.card_gap),
                        cx,
                    )
                    .into_any_element()
                }
            };
            let is_held = matches!(&keys[at], ItemKey::Row(id) if Some(*id) == held);
            items.push(self.with_list_motion(keys[at].clone(), line, is_held));
        }
        // After the person's own terminals: the welcome's example group, which is none of them.
        items.extend(self.render_demo(&frame, cx));

        div()
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .children(glass.as_ref().map(|glass| clip_probe(glass, LIST_CLIP)))
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
                    .track_scroll(&self.scroll)
                    .children(items);
                let list = self.with_list_drop(list.child(self.list_tail(cx)), cx);
                // It holds the keyboard for the sidebar.
                self.area_container(list, cx)
            })
            // Beside the list, not in it: a child of the scrolled list would move with its rows.
            .child(self.list_edges(ui, cx))
            .into_any_element()
    }

    /// What lies over the list's edges (`panel_list::scroll_edges`, as the panel's lists).
    fn list_edges(&self, ui: UiColors, cx: &App) -> AnyElement {
        let scrollbar = Scrollbar::vertical(&self.scroll).id("sidebar-scrollbar");
        crate::window::panel_list::scroll_edges(
            ui,
            crate::window::panel_list::is_scrolled(&self.scroll),
            scrollbar,
            cx,
        )
    }
}

/// The window draws the list as a child view, after it has told it how it stands (`set_input`): a child
/// renders once its parent's render has returned, so this never meets the window's lease.
impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_list(window, cx)
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
