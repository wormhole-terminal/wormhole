//! The terminal list on the left. Core
//! (`wormhole_core::sidebar`) decides what is listed, in what order, with which ⌘ numbers
//! and in which words; this gathers each terminal's facts, draws the bands and rows, and
//! wires up the ⌘-hold hints (`hints`), the filter popover (`filter_popover`), the row and
//! band menus (`row_menu`, `band_menu`) and drag and drop (`drag`).
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::glass::{GlassKind, Insets, LIST_CLIP, glass_key};
use super::session::home_dir;
use super::*;

use wormhole_core::agents::AgentId;
use wormhole_core::git::HeadCommit;
use wormhole_core::sidebar::NUMBERED;
use wormhole_core::sidebar::hints::CmdHold;
use wormhole_core::sidebar::order::{FrozenNumbers, settle_bands};
use wormhole_core::sidebar::{
    Entry, ListInput, Listing, RowState, Searchable, SessionGit, SidebarNumbers, SidebarOrder, Splits,
    WindowSidebar,
};

mod band_menu;
mod bands;
mod demo;
mod drag;
mod facts;
mod filter_popover;
mod groups;
mod hints;
mod keys;
mod motion;
mod row_menu;
mod rows;
mod tab_bar;
mod tick;

/// The room a loose terminal's glass leaves above and below it.
const LOOSE_GAP: f32 = 2.0;

use facts::{RowInfo, row_times};
use motion::{ItemKey, ListMotion};
pub(super) use row_menu::reveal_in_finder;

/// Whether the funnel and what it filters exist (the `SidebarFilter` flag). Off, the list is
/// built as if no filter were saved, and nothing opens the popover.
pub(crate) fn sidebar_filter_on(cx: &App) -> bool {
    settings(cx).feature_enabled(wormhole_core::features::FeatureFlag::SidebarFilter)
}

/// The sidebar's part of a window: what is saved with it (grouping, filter, folded bands) and
/// what it is doing right now.
pub(super) struct SidebarState {
    /// Saved in the window's layout.
    pub(super) view: WindowSidebar,
    /// The filter popover's search field; its text is `view.filter.search`.
    pub(super) search: Entity<InputState>,
    _search: Subscription,
    pub(super) filter_open: bool,
    /// ⌘ held on its own (the ⌘1–9 keycaps), and the timer that shows them.
    hold: CmdHold,
    hold_timer: Option<Task<()>>,
    _keys: Subscription,
    /// The terminal whose context menu is open: its row wears the focus ring.
    menu_row: Option<i64>,
    /// The folder whose band's context menu is open: the band wears the ring.
    menu_band: Option<PathBuf>,
    /// The group whose band's context menu is open.
    menu_group: Option<u32>,
    /// The group whose name is being edited in place.
    renaming: Option<groups::Renaming>,
    menu_dismissed: Option<Subscription>,
    pub(super) scroll: ScrollHandle,
    /// Bring this terminal's row into view on the next frame.
    pub(super) scroll_to: Option<i64>,
    /// The row that just moved to another band, flashed once: its terminal and a counter that
    /// restarts the flash.
    pub(super) flash: Option<(i64, u64)>,
    /// The selected terminal's band as last seen, to notice it moving.
    active_band: Option<(i64, Option<PathBuf>)>,
    /// Detached HEADs' commits by repository, looked up off the UI thread: when, and what.
    heads: HashMap<PathBuf, (u64, Option<HeadCommit>)>,
    head_lookups: Vec<Task<()>>,
    /// The order the folder bands keep under the stable order: a band stays where it appeared
    /// while it has a terminal (`order::settle_bands`).
    band_order: RefCell<Vec<PathBuf>>,
    /// The ⌘ numbers frozen to their terminals (by id) under "Stay with the terminal".
    numbers: RefCell<FrozenNumbers<i64>>,
    /// The list's lines as last drawn and the slide of those that moved.
    motion: ListMotion,
    /// Which repository a worktree belongs to, by its root (read from disk once per refresh).
    worktrees: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
    /// The times the rows showed when last drawn ("Working · 3m", "1:12"), so a tick redraws
    /// only when one of them would read differently.
    times_drawn: RefCell<Vec<Option<String>>>,
    /// The terminal welcome's example band while its sidebar step is up (`demo`). Never saved.
    pub(super) demo: Option<demo::Demo>,
    /// The tab bar, while the terminals are in tabs.
    pub(super) tabs: tab_bar::TabBarState,
}

impl SidebarState {
    pub(super) fn new(view: WindowSidebar, window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search name, folder or branch")
                .default_value(view.filter.search.clone())
        });
        let search_events = cx.subscribe_in(&search, window, |this, input, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                let text = input.read(cx).value().to_string();
                if this.sidebar.view.filter.search != text {
                    this.sidebar.view.filter.search = text;
                    this.mark_dirty(cx);
                }
            }
        });
        let keys = hints::watch_keys(window, cx);
        Self {
            view,
            search,
            _search: search_events,
            filter_open: false,
            hold: CmdHold::default(),
            hold_timer: None,
            _keys: keys,
            menu_row: None,
            menu_band: None,
            menu_group: None,
            renaming: None,
            menu_dismissed: None,
            scroll: ScrollHandle::new(),
            scroll_to: None,
            flash: None,
            active_band: None,
            heads: HashMap::new(),
            head_lookups: Vec::new(),
            band_order: RefCell::default(),
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
        let folders: Vec<Option<PathBuf>> = match self.sidebar.view.grouping {
            Grouping::Folder => infos.iter().map(|info| info.folder.clone()).collect(),
            Grouping::None | Grouping::Custom => Vec::new(),
        };
        // The person's pinned folders, one list for every window (a flat list has no bands).
        let pinned_folders: Vec<PathBuf> = match self.sidebar.view.grouping {
            Grouping::Folder => settings(cx).pinned_folders.iter().map(PathBuf::from).collect(),
            Grouping::None | Grouping::Custom => Vec::new(),
        };
        let group_of: Vec<Option<u32>> = match self.sidebar.view.grouping {
            Grouping::Custom => infos.iter().map(|info| self.sidebar.view.groups.group_of(info.id)).collect(),
            Grouping::None | Grouping::Folder => Vec::new(),
        };
        let rows: Vec<Searchable> = infos.iter().map(RowInfo::searchable).collect();
        let pinned: Vec<bool> = infos.iter().map(|info| info.pinned).collect();
        let split_of = self.split_ids();
        let view = self.sidebar.view.as_seen(sidebar_filter_on(cx));
        let (order, numbers) = (settings(cx).sidebar_order, settings(cx).sidebar_numbers);
        let band_order = match (self.sidebar.view.grouping, order) {
            (Grouping::Folder, SidebarOrder::Stable) => {
                let settled = settle_bands(&self.sidebar.band_order.borrow(), &folders);
                self.sidebar.band_order.replace(settled.clone());
                settled
            }
            _ => Vec::new(),
        };
        let mut listing = Listing::build(&ListInput {
            grouping: self.sidebar.view.grouping,
            folders: &folders,
            pinned_folders: &pinned_folders,
            rows: &rows,
            pinned: &pinned,
            sidebar: &view,
            home: home_dir(),
            splits: Splits { split_of: &split_of, selected: Some(self.active) },
            order,
            numbers,
            band_order: &band_order,
            group_of: &group_of,
        });
        self.freeze_numbers(&mut listing, infos, order, cx);
        listing
    }

    /// Under "Stay with the terminal", the numbers the listing dealt are replaced by the frozen ones:
    /// kept while the terminals are only added to, dealt again when the person changes the order,
    /// closes one or changes a setting that orders the list (`FrozenNumbers`).
    fn freeze_numbers(&self, listing: &mut Listing, infos: &[RowInfo], order: SidebarOrder, cx: &App) {
        let mut frozen = self.sidebar.numbers.borrow_mut();
        if settings(cx).sidebar_numbers != SidebarNumbers::Terminal {
            frozen.reset();
            return;
        }
        let tag = u8::from(self.sidebar.view.grouping == Grouping::Folder)
            | (u8::from(order == SidebarOrder::Classic) << 1);
        let window: Vec<(i64, bool)> = infos.iter().map(|info| (info.id, info.pinned)).collect();
        let dealt: Vec<i64> = listing.order().iter().map(|&ix| infos[ix].id).collect();
        let held = frozen.settle(&window, tag, &dealt, NUMBERED);
        let at = |id: &i64| infos.iter().position(|info| info.id == *id);
        listing.set_numbers(held.iter().filter_map(at).collect());
    }

    /// The listing without drawing it (shortcuts, devctl).
    pub(super) fn session_listing(&self, cx: &App) -> Listing {
        self.listing(&self.row_infos(cx), cx)
    }

    /// Every terminal top to bottom as the sidebar lists it (folded and filtered ones included).
    pub(super) fn session_order(&self, cx: &App) -> Vec<usize> {
        self.session_listing(cx).order().to_vec()
    }

    pub(crate) fn set_grouping(&mut self, grouping: Grouping, cx: &mut Context<Self>) {
        if self.sidebar.view.grouping != grouping {
            self.sidebar.view.grouping = grouping;
            self.mark_dirty(cx);
        }
    }

    /// Fold an open band, or open a folded one.
    pub(super) fn toggle_band(&mut self, folder: Option<PathBuf>, cx: &mut Context<Self>) {
        self.sidebar.view.toggle_fold(folder.as_deref());
        self.mark_dirty(cx);
    }

    /// A new terminal in a band's folder (the band's +).
    pub(crate) fn new_session_in(
        &mut self,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = folder.map(|folder| folder.to_string_lossy().into_owned()).or_else(|| self.default_cwd(cx));
        self.open_session(None, cwd, window, cx);
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
        let git = self.session_git(cx);
        let held = cx.has_active_drag().then(|| self.terminal_drag.as_ref().map(|drag| drag.id)).flatten();
        let mut items: Vec<AnyElement> = Vec::with_capacity(listing.entries.len());
        for (at, entry) in listing.entries.iter().enumerate() {
            let line = match *entry {
                Entry::Band { band, position, of } if listing.bands[band].group.is_some() => {
                    self.render_group_band(&listing, band, (position, of), &frame, false, window, cx)
                }
                Entry::Band { band, position, of } => {
                    self.render_band(&listing, band, (position, of), &git, &frame, false, false, window, cx)
                }
                Entry::Row { session, band, position, of, number, marks } => {
                    let place = rows::Place {
                        position,
                        of,
                        number,
                        band: band.and_then(|b| listing.bands[b].folder.clone()),
                        grouped: band.is_some(),
                        marks,
                    };
                    self.render_row(&infos[session], &place, &listing, &frame, window, cx)
                }
            };
            // Each line is a stretch of its band's card; a loose terminal is lined up with the cards.
            let (in_card, top) = match *entry {
                Entry::Band { .. } => (true, true),
                Entry::Row { band, .. } => (band.is_some(), false),
            };
            let card_of = |entry: &Entry| match *entry {
                Entry::Band { band, .. } | Entry::Row { band: Some(band), .. } => Some(band),
                Entry::Row { band: None, .. } => None,
            };
            let own = card_of(entry);
            let bottom = in_card
                && !matches!(listing.entries.get(at + 1), Some(next @ Entry::Row { .. }) if card_of(next) == own);
            // A card after a loose terminal stands off from it; after another card, that card's gap.
            let gap_above =
                at > 0 && matches!(listing.entries.get(at - 1), Some(Entry::Row { band: None, .. }));
            let segment = style::card_segment(ui, top, bottom, in_card, gap_above, line);
            // Each card, and each loose terminal, is a glass of its own.
            let segment = if glass {
                let (key, inset) = match (own, *entry) {
                    (Some(band), _) => (
                        glass_key(GlassKind::Card, 1 + band as u64),
                        Insets(
                            style::CARD_MARGIN,
                            if top && gap_above { style::CARD_GAP } else { 0.0 },
                            style::CARD_MARGIN,
                            if bottom { style::CARD_GAP } else { 0.0 },
                        ),
                    ),
                    (None, Entry::Row { session, .. }) => (
                        glass_key(GlassKind::Loose, infos[session].id as u64),
                        Insets(style::CARD_MARGIN, LOOSE_GAP, style::CARD_MARGIN, LOOSE_GAP),
                    ),
                    (None, Entry::Band { .. }) => (0, Insets(0.0, 0.0, 0.0, 0.0)),
                };
                segment.child(self.glass_probe(key, inset, style::CARD_RADIUS, Some(LIST_CLIP)))
            } else {
                segment
            };
            let line = match *entry {
                Entry::Row { session, .. } => {
                    let id = infos[session].id;
                    self.with_row_drop(segment.id(("row-line", id as u64)), id, cx).into_any_element()
                }
                Entry::Band { .. } => segment.into_any_element(),
            };
            let is_held = matches!(&keys[at], ItemKey::Row(id) if Some(*id) == held);
            items.push(self.with_list_motion(keys[at].clone(), line, is_held));
        }
        // After the person's own terminals: the welcome's example band, which is none of them.
        items.extend(self.render_demo(&frame, cx));
        let filtered = self.sidebar.view.as_seen(sidebar_filter_on(cx)).filter.is_active();
        let sticky = self.sticky_band(&listing, &git, &frame, window, cx);
        let empty = listing.entries.is_empty() && filtered && !self.sessions.is_empty();

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
            .child(self.render_sidebar_header(listing.idle, window, cx))
            .children(self.filter_chip(cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    // The stuck band is pushed up out of the list by the next one, not over the header.
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
                            .children(items)
                            .when(filtered && listing.hidden > 0 && !empty, |d| {
                                d.child(self.hidden_line(listing.hidden, cx))
                            });
                        let last = listing.entries.iter().rev().find_map(|entry| match *entry {
                            Entry::Row { session, .. } => Some(infos[session].id),
                            Entry::Band { .. } => None,
                        });
                        let list = self.with_list_drop(list.child(self.list_tail(last, cx)), cx);
                        // It holds the keyboard for the sidebar.
                        self.area_container(wormhole_core::focus::Area::Sidebar, list, cx)
                    })
                    .children(sticky)
                    .when(empty, |d| d.child(self.no_match(cx))),
            )
            .children(self.render_no_agents_note(cx))
            .into_any_element()
    }

    /// Fold the band called `label`, or open it; `false` if there is none.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_fold(&mut self, label: &str, cx: &mut Context<Self>) -> bool {
        let listing = self.session_listing(cx);
        let Some(band) = listing.bands.iter().find(|band| band.label == label) else { return false };
        self.toggle_band(band.folder.clone(), cx);
        true
    }

    /// Pin the folder, or unpin it; whether it is pinned now.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_pin_folder(&mut self, folder: &Path, cx: &mut Context<Self>) -> bool {
        band_menu::toggle_pinned_folder(folder, cx);
        settings(cx).is_folder_pinned(folder)
    }

    /// A new group named `name` holding the terminals at `rows` (as `state` numbers them, from 0),
    /// shown as groups; its id.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_new_group(&mut self, name: &str, rows: &[usize], cx: &mut Context<Self>) -> u32 {
        let group = self.sidebar.view.groups.create(&[]);
        self.sidebar.view.groups.rename(group, name);
        for &ix in rows {
            if let Some(id) = self.sessions.get(ix).map(|session| session.id) {
                self.move_to_group(id, Some(group), cx);
            }
        }
        self.set_grouping(Grouping::Custom, cx);
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

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_toggle_filter_popover(&mut self, cx: &mut Context<Self>) {
        if !sidebar_filter_on(cx) {
            return;
        }
        self.sidebar.filter_open = !self.sidebar.filter_open;
        cx.notify();
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
const LABEL_REM: f32 = 0.8125;
const CAPTION_REM: f32 = 0.75;
