//! The terminal list on the left, an entity of its own (`Sidebar`) that the window hosts. Core
//! (`wormhole_core::view::sidebar`) decides what is listed, in what order, with which ⌘ numbers
//! and in which words; this holds the list's state, draws the groups and rows, and wires up the
//! ⌘-hold hints (`hints`), the row and group menus (`row_menu`, `groups`) and drag and drop (`drop`).
//!
//! The sidebar never reads the `Workspace`. The window pushes it a [`SidebarInput`] (the terminals as
//! rows, which is selected, the profile, the width, the glass, the drag in flight) before it is drawn
//! and before the window asks it for something that depends on the terminals as they are; what the
//! person does in the list comes back as a [`SidebarEvent`] the window handles (`window::sidebar_host`).
//! The window reaches the list's state through the methods marked `pub(crate)`.
use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::focus::list::TypeAhead;
use wormhole_core::view::sidebar::NUMBERED;
use wormhole_core::view::sidebar::hints::CmdHold;
use wormhole_core::view::sidebar::order::FrozenNumbers;
use wormhole_core::view::sidebar::{ListInput, Listing, SidebarNumbers, Splits, WindowSidebar};

use crate::settings::{UiColors, settings_in};
use crate::sidebar::terminal_drag::Over;
use crate::style::TextRole;

mod card;
mod demo;
mod drop;
mod facts;
mod glass;
pub(crate) use self::glass::{column_end, column_start, forget_column};
mod group_head;
mod group_popover;
mod groups;
mod hints;
mod input;
mod keys;
mod list;
mod motion;
pub(crate) mod profile_chip;
mod rename;
mod row;
mod row_menu;
pub(crate) mod shells;
mod switcher;
mod tab_bar;
pub(crate) mod tear_off;
pub(crate) mod terminal_drag;

pub(crate) use facts::row_times;
pub(crate) use facts::{RowExtra, RowExtras, RowFailure, RowInfo};
pub(crate) use input::{NewTerminal, SidebarEvent, SidebarInput};
pub(crate) use keys::SidebarStop;
use motion::{ItemKey, ListMotion, slot_id};
pub(crate) use row_menu::reveal_in_finder;
pub(crate) use tab_bar::TAB_ROW_H;

/// The terminal list: what is saved with the window (the groups, their order, which are folded) and what
/// it is doing right now.
pub(crate) struct Sidebar {
    /// Saved in the window's layout.
    view: WindowSidebar,
    /// What the window last said of itself.
    input: Rc<SidebarInput>,
    /// The column's width, and the window's: layout (`set_metrics`), not facts.
    width: f32,
    window_width: f32,
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
    /// What it was over the last time it was over anything, which a line's edge holds it in (`drop`).
    over_before: Option<Over>,
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
    /// Where the chip reads the app's profiles from.
    profiles: profile_chip::ProfilesFn,
    /// How many times the list was built, for the tests that check a notification drew it (and
    /// devctl's report).
    #[cfg(any(test, feature = "devctl"))]
    built: usize,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    /// A list with `view` saved from the window it was in, whose keys go to `focus` (the area's handle).
    pub(crate) fn new(
        view: WindowSidebar,
        focus: FocusHandle,
        profiles: profile_chip::ProfilesFn,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let keys = hints::watch_keys(window, cx);
        Self {
            view,
            input: Rc::default(),
            width: 0.0,
            window_width: f32::MAX,
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
            over_before: None,
            order_rev: 0,
            numbers: RefCell::default(),
            motion: ListMotion::default(),
            times_drawn: Vec::new(),
            demo: None,
            tabs: tab_bar::TabBarState::default(),
            profile_menu: None,
            profiles,
            #[cfg(any(test, feature = "devctl"))]
            built: 0,
        }
    }

    /// What the window says of itself now: what the next frame, and every call that reads the
    /// terminals, goes by.
    ///
    /// Draws again when it differs from what it was told last; the window says so when its facts
    /// change (`window::sidebar_host`), never while it draws.
    pub(crate) fn set_input(&mut self, input: SidebarInput, cx: &mut Context<Self>) -> bool {
        if *self.input == input {
            return false;
        }
        self.input = Rc::new(input);
        cx.notify();
        true
    }

    /// What the window last said, to compare against what it would say now.
    #[cfg(test)]
    pub(crate) fn input(&self) -> &SidebarInput {
        &self.input
    }

    /// The column's width and the window's as last said (`set_metrics`).
    pub(crate) fn metrics(&self) -> (f32, f32) {
        (self.width, self.window_width)
    }

    /// The column's width (its width at rest while it slides) and the window's, which the tab bar
    /// fits itself to: layout, known when the window lays out. Draws again when one changed.
    pub(crate) fn set_metrics(&mut self, width: f32, window_width: f32, cx: &mut Context<Self>) {
        if (self.width, self.window_width) != (width, window_width) {
            (self.width, self.window_width) = (width, window_width);
            cx.notify();
        }
    }

    /// The saved view: the groups, their order, which are folded.
    pub(crate) fn view(&self) -> &WindowSidebar {
        &self.view
    }

    /// The saved view, to change (the window seats a joined terminal beside its mates, closes one).
    /// The caller says `Changed` itself when it matters.
    pub(crate) fn view_mut(&mut self) -> &mut WindowSidebar {
        &mut self.view
    }

    /// The list's scroll, which devctl and the tests turn and measure.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn scroll(&self) -> &ScrollHandle {
        &self.scroll
    }

    /// What the pointer is over in the list while something is dragged over it.
    #[cfg(test)]
    pub(crate) fn over(&self) -> Option<Over> {
        self.over
    }

    /// The pointer left the list for the panes: nothing in the list is the target. Whether it was.
    pub(crate) fn take_over(&mut self) -> bool {
        self.over.take().is_some()
    }

    /// The list's focus handle, which holds the keyboard while the list is the area the keys are in.
    pub(crate) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// The item the keys are on.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn keys_at(&self) -> Option<&SidebarStop> {
        self.keys_at.as_ref()
    }

    /// Whether any row slides to a new place now (the list's move animation under way).
    #[cfg(test)]
    pub(crate) fn rows_moving(&self) -> bool {
        self.motion.is_moving()
    }

    /// Whether any line is in a motion now: sliding, coming in, leaving or fading in a summary.
    #[cfg(test)]
    pub(crate) fn motion_busy(&self) -> bool {
        self.motion.is_busy()
    }

    /// The terminals whose rows a dragged row pushes aside now, and how far (down is positive).
    #[cfg(test)]
    pub(crate) fn pushed_rows(&self) -> Vec<(i64, f32)> {
        self.motion.pushed_rows()
    }

    /// The terminals whose rows are out of the list while a gap is open for them.
    #[cfg(test)]
    pub(crate) fn lifted_rows(&self) -> Vec<i64> {
        self.motion.lifted_rows()
    }

    /// What a split's preview is doing: the row the dragged one would join, the row whose preview is going
    /// out, whether the chip snaps onto a row.
    #[cfg(test)]
    pub(crate) fn join_state(&self) -> (Option<i64>, Option<i64>, bool) {
        self.join_preview_state()
    }

    /// The terminals whose rows draw their bracket in now.
    #[cfg(test)]
    pub(crate) fn bracket_rows(&self) -> Vec<i64> {
        self.join_bracket_rows()
    }

    /// What the last plan of the list moved: each row's terminal, where it started and where it went.
    #[cfg(test)]
    pub(crate) fn planned_rows(&self) -> Vec<(i64, f32, f32)> {
        self.motion.planned_rows()
    }

    /// Whether a drag's gap is held (open, or closing).
    #[cfg(test)]
    pub(crate) fn has_room(&self) -> bool {
        self.motion.has_room()
    }

    /// Whether each row that is leaving was the terminal on screen when it was last drawn.
    #[cfg(test)]
    pub(crate) fn leaving_selected(&self) -> Vec<(i64, bool)> {
        self.motion.leaving_selected()
    }

    /// The terminals whose tile's status is changing, and how it is played.
    #[cfg(test)]
    pub(crate) fn statuses_playing(&self) -> Vec<(i64, wormhole_core::view::motion::StatusMotion)> {
        self.motion.statuses.playing()
    }

    /// What terminal `id`'s tile says now, as an urgency.
    #[cfg(test)]
    pub(crate) fn shown_urgency(&self, id: i64) -> Option<wormhole_core::view::sidebar::Urgency> {
        let info = self.input.rows.iter().find(|row| row.id == id)?;
        Some(row::glyph_for(info).into())
    }

    /// Whether any tile's status is changing.
    #[cfg(test)]
    pub(crate) fn statuses_busy(&self) -> bool {
        self.motion.statuses.is_playing()
    }

    /// The terminals whose rows have left the list and are still drawn while they go.
    #[cfg(test)]
    pub(crate) fn leaving_ids(&self) -> Vec<i64> {
        self.motion.leaving_ids()
    }

    /// The terminal whose row wears the ring for an open menu.
    #[cfg(test)]
    pub(crate) fn menu_row(&self) -> Option<i64> {
        self.menu_row
    }

    /// The group being renamed, and its name field.
    #[cfg(test)]
    pub(crate) fn group_renaming(&self) -> Option<(u32, Entity<gpui_kit::component::input::InputState>)> {
        self.group_renaming.as_ref().map(|r| (r.group, r.input.clone()))
    }

    /// Whether the terminal welcome's example band is up.
    #[cfg(test)]
    pub(crate) fn has_demo(&self) -> bool {
        self.demo.is_some()
    }

    /// How many times the list was built.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn builds(&self) -> usize {
        self.built
    }

    /// The profile chip's menu is open.
    #[cfg(test)]
    pub(crate) fn profile_menu_open(&self) -> bool {
        self.profile_menu.is_some()
    }

    /// Open the profile chip's menu, as a click on the chip does.
    #[cfg(test)]
    pub(crate) fn open_profile_menu_for_test(&mut self, cx: &mut Context<Self>) {
        self.profile_menu = Some(profile_chip::ProfilePage::Main);
        cx.notify();
    }

    /// The page of the profile chip's menu that is open. The chip is drawn by the window's header, a view
    /// of its own, so the window watches this to draw the header again (`Regions::watch_sidebar`).
    pub(crate) fn profile_page(&self) -> Option<profile_chip::ProfilePage> {
        self.profile_menu
    }

    /// The terminal being renamed, and its name field.
    #[cfg(test)]
    pub(crate) fn terminal_renaming(&self) -> Option<Entity<gpui_kit::component::input::InputState>> {
        self.renaming.as_ref().map(|r| r.input.clone())
    }

    /// Whether the times on the rows read as `now` says: no, and the list wants a redraw (a row's
    /// "Working · 3m" moved on its own).
    pub(crate) fn times_changed(&mut self, now: Vec<Option<String>>) -> bool {
        if self.times_drawn == now {
            return false;
        }
        self.times_drawn = now;
        true
    }

    /// The list as the sidebar draws it, from `input`'s terminals.
    pub(crate) fn listing_of(&self, input: &SidebarInput, cx: &App) -> Listing {
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
    pub(crate) fn list_rearranged(&mut self, cx: &mut Context<Self>) {
        self.order_rev += 1;
        self.changed(cx);
    }

    /// The saved view changed: tell the window to save it, and draw.
    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(SidebarEvent::Changed);
        cx.notify();
    }
}

/// The window draws the list as a child view, from what it told it of how it stands (`set_input`): a child
/// renders once its parent's render has returned, so this never meets the window's lease.
impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(any(test, feature = "devctl"))]
        {
            self.built += 1;
        }
        let glass = self.input.glass.clone();
        // The list's glass is bracketed here, in the view that proposes it: GPUI replays the list
        // (it is cached, a view of its own) in frames that change none of it, and the glass keeps
        // what it proposed last until it proposes again. `size_full` fills the bounds a cached view is
        // given; `flex_1` is the share of the column when the view is drawn inline (a sliding column).
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .size_full()
            .flex()
            .flex_col()
            .children(glass.as_ref().map(column_start))
            .child(self.render_list(window, cx))
            .children(glass.as_ref().map(column_end))
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
