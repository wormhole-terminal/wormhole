//! The terminal list on the left. Core
//! (`wormhole_core::sidebar`) decides what is listed, in what order, with which ⌘ numbers
//! and in which words; this gathers each terminal's facts, draws the bands and rows, and
//! wires up the ⌘-hold hints (`hints`), the filter popover (`filter_popover`), the row and
//! band menus (`row_menu`, `band_menu`) and drag and drop (`drag`).
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::session::home_dir;
use super::*;

use wormhole_core::agents::AgentId;
use wormhole_core::git::HeadCommit;
use wormhole_core::sidebar::hints::CmdHold;
use wormhole_core::sidebar::{
    Entry, ListInput, Listing, RowState, Searchable, SessionGit, Splits, WindowSidebar,
};

mod band_menu;
mod bands;
mod demo;
mod drag;
mod facts;
mod filter_popover;
mod hints;
mod keys;
mod row_menu;
mod rows;
mod tick;

use facts::{RowInfo, row_times};
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
    /// Which repository a worktree belongs to, by its root (read from disk once per refresh).
    worktrees: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
    /// The times the rows showed when last drawn ("Working · 3m", "1:12"), so a tick redraws
    /// only when one of them would read differently.
    times_drawn: RefCell<Vec<Option<String>>>,
    /// The terminal welcome's example band while its sidebar step is up (`demo`). Never saved.
    pub(super) demo: Option<demo::Demo>,
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
            menu_dismissed: None,
            scroll: ScrollHandle::new(),
            scroll_to: None,
            flash: None,
            active_band: None,
            heads: HashMap::new(),
            head_lookups: Vec::new(),
            worktrees: RefCell::default(),
            times_drawn: RefCell::default(),
            demo: None,
        }
    }
}

impl Workspace {
    /// The list as the sidebar draws it.
    fn listing(&self, infos: &[RowInfo], cx: &App) -> Listing {
        let folders: Vec<Option<PathBuf>> = match self.sidebar.view.grouping {
            Grouping::Folder => infos.iter().map(|info| info.folder.clone()).collect(),
            Grouping::None => Vec::new(),
        };
        // The person's pinned folders, one list for every window (a flat list has no bands).
        let pinned_folders: Vec<PathBuf> = match self.sidebar.view.grouping {
            Grouping::Folder => settings(cx).pinned_folders.iter().map(PathBuf::from).collect(),
            Grouping::None => Vec::new(),
        };
        let rows: Vec<Searchable> = infos.iter().map(RowInfo::searchable).collect();
        let pinned: Vec<bool> = infos.iter().map(|info| info.pinned).collect();
        let split_of = self.split_ids();
        let view = self.sidebar.view.as_seen(sidebar_filter_on(cx));
        Listing::build(&ListInput {
            grouping: self.sidebar.view.grouping,
            folders: &folders,
            pinned_folders: &pinned_folders,
            rows: &rows,
            pinned: &pinned,
            sidebar: &view,
            home: home_dir(),
            splits: Splits { split_of: &split_of, selected: Some(self.active) },
        })
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
        let frame = rows::Frame::new(self, width, window, cx);
        let git = self.session_git(cx);
        let mut items: Vec<AnyElement> = Vec::with_capacity(listing.entries.len());
        for (at, entry) in listing.entries.iter().enumerate() {
            items.push(match *entry {
                Entry::Band { band, position, of } => {
                    // Two bands in a row (the first folded) share one hairline.
                    let joined = at > 0 && matches!(listing.entries[at - 1], Entry::Band { .. });
                    self.render_band(&listing, band, (position, of), &git, &frame, false, joined, window, cx)
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
            });
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
            .bg(ui.panel)
            .child(self.render_sidebar_header(listing.idle, window, cx))
            .children(self.filter_chip(cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    // The stuck band is pushed up out of the list by the next one, not over the header.
                    .overflow_hidden()
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
