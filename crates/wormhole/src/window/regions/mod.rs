//! The window's chrome as views GPUI keeps between frames: the sidebar's header, the right panel,
//! the terminal's title row and the status bar. The sidebar's list is such a view too, but its own (it
//! draws itself), and the sidebar's column lays the two out side by side.
//!
//! GPUI draws the window's root again for every frame a terminal streams, and with it every div the
//! chrome is made of, although a terminal's output changes none of it. A [`Region`] is a view that draws
//! one part of the chrome through the workspace's own builders, and is laid out `cached`: a frame that
//! changes nothing in it replays what it drew last time (bounds, content mask and text style
//! unchanged, GPUI's rule), without building, laying out or painting it again.
//!
//! What makes it draw again is the workspace being notified, or one of the globals the parts read
//! changing ([`Regions::watch_globals`]). Both reach the regions (and the list) outside a draw, which is the
//! only time GPUI acts on a notification for a view it replays. The facts the parts show reach the
//! workspace already: a terminal's summary changing, a hook, the tick comparing the words that move
//! with time (`refresh_status_bar`, `crumb_tick`, the sidebar's `times_changed`), a feature's entity
//! changing (`feature_host`) and the settings. A new fact a part draws must notify the workspace the
//! same way; one that only changes in a draw would be missed.
//!
//! While an app has asked for the window's accessibility tree the regions stay cached: GPUI rebuilds the
//! tree in every frame, and replays a reused view's nodes and action listeners with the rest of it (the
//! patched `gpui-pre`, `window::cached`). On macOS the tree, once on, stays on for the window's life, and any
//! app using the accessibility API turns it on (`platform::a11y::tree_built`), so the window logs it once
//! ([`Regions::note_tree`]): a profile of it should know the tree is being built.
//!
//! A missed notify shows nowhere else: a test that draws drops GPUI's caches first. In debug and test builds
//! every region records what it was built from and every frame checks that the regions it placed would be
//! built from the same now (`oracle`), and the workspace may not be notified while a frame draws, when
//! nothing listens (`Oracle::assert_not_drawing`).
//!
//! A change of focus, the keyboard modality, the window's size or activation and every click, drag
//! and tooltip make GPUI drop all its caches for a frame (`Window::refresh`), so those need no notify.

mod app_facts;
#[cfg(any(test, debug_assertions))]
mod oracle;

use std::cell::Cell;

use self::app_facts::AppFacts;
use wormhole_core::view::sidebar::GroupPaint;

use gpui_kit::*;
use wormhole_core::view::layout::status_bar::STATUS_BAR_H;

use crate::sidebar::Sidebar;
use crate::sidebar::profile_chip::ProfilePage;
use crate::window::Workspace;
use crate::window::status_bar::StatusBar;
use crate::window::status_rolls::StatusRolls;

/// A part of the chrome that is a view of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// The sidebar's header: the traffic lights' row with the profile chip and the + for a new terminal.
    /// Not the list under it (a view of its own, so that its drawing leaves this alone) nor the first-run
    /// note under that (as tall as its words, which a replayed view cannot be).
    SidebarHeader,
    /// The right panel's column: its tab bar over the shown tab.
    Panel,
    /// The terminal's title row.
    TitleRow,
    /// The optional bar along the bottom.
    StatusBar,
}

impl Part {
    /// How many there are.
    #[cfg(any(test, debug_assertions))]
    const COUNT: usize = 4;
    /// Every part, in the order of [`Part::index`].
    #[cfg(any(test, debug_assertions))]
    const ALL: [Part; Self::COUNT] = [Part::SidebarHeader, Part::Panel, Part::TitleRow, Part::StatusBar];

    /// Where the part is in arrays kept per part.
    #[cfg(any(test, debug_assertions))]
    const fn index(self) -> usize {
        self as usize
    }
}

/// One part of the chrome, drawn from the workspace it belongs to.
pub(super) struct Region {
    part: Part,
    workspace: WeakEntity<Workspace>,
    /// How many times it was built, for the tests that check a frame left it alone.
    #[cfg(any(test, feature = "devctl"))]
    builds: usize,
}

impl Render for Region {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(any(test, feature = "devctl"))]
        {
            self.builds += 1;
        }
        let part = self.part;
        // The workspace is not being updated: a view is built after its parent's render has returned.
        match self.workspace.upgrade() {
            Some(workspace) => workspace.update(cx, |workspace, cx| workspace.render_part(part, window, cx)),
            None => div().into_any_element(),
        }
    }
}

/// The window's regions.
pub(super) struct Regions {
    sidebar_header: Entity<Region>,
    /// The terminal list, drawn beside the header and replayed as the regions are, but a view that
    /// draws itself: a notification of the list (a hover, a spinner's turn) must not reach the header.
    list: Entity<Sidebar>,
    panel: Entity<Region>,
    title_row: Entity<Region>,
    status_bar: Entity<Region>,
    /// The accessibility tree was built in some frame of this window, so the chrome has not been cached since.
    tree_seen: Cell<bool>,
    /// What the regions last drew of what moves with time, to notice on the tick when it is stale.
    pub(super) drawn: Drawn,
    /// The shared state's facts as the window last saw them (`watch_globals`).
    app_seen: Option<AppFacts>,
    /// What the selected terminal's group was last painted with, as the list's notifications saw it.
    paint_seen: Option<GroupPaint>,
    /// Which page of the profile chip's menu was open, as the list's notifications saw it.
    menu_seen: Option<ProfilePage>,
    /// What each was built from, checked at the end of every frame.
    #[cfg(any(test, debug_assertions))]
    oracle: oracle::Oracle,
}

/// What the regions drew last of what changes with the clock.
#[derive(Default)]
pub(super) struct Drawn {
    /// The status bar (`None` while it is off).
    pub(super) status_bar: Option<StatusBar>,
    /// The breadcrumb's status words ("working · 4m"), for the minute.
    pub(super) crumb: Option<String>,
    /// The bar's counts that are turning over.
    pub(super) rolls: StatusRolls,
}

impl Regions {
    pub(super) fn new(workspace: WeakEntity<Workspace>, list: Entity<Sidebar>, cx: &mut App) -> Self {
        let mut region = |part| {
            let workspace = workspace.clone();
            cx.new(|_| Region {
                part,
                workspace,
                #[cfg(any(test, feature = "devctl"))]
                builds: 0,
            })
        };
        Self {
            sidebar_header: region(Part::SidebarHeader),
            list,
            panel: region(Part::Panel),
            title_row: region(Part::TitleRow),
            status_bar: region(Part::StatusBar),
            tree_seen: Cell::new(false),
            drawn: Drawn::default(),
            app_seen: None,
            paint_seen: None,
            menu_seen: None,
            #[cfg(any(test, debug_assertions))]
            oracle: Default::default(),
        }
    }

    /// A frame begins in window `window_id`: say once if the accessibility tree is built in it. From then on
    /// every frame also builds the tree (see the module's note), which a profile of the window should know.
    pub(super) fn note_tree(&self, window: &Window, window_id: i64) {
        if crate::platform::a11y::tree_built(window) && !self.tree_seen.replace(true) {
            tracing::info!(
                window_id,
                "an app asked for the accessibility tree: every frame of this window now builds it"
            );
        }
    }

    /// Whether the accessibility tree has been built in this window (every frame builds it from then on).
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn tree_seen(&self) -> bool {
        self.tree_seen.get()
    }

    fn all(&self) -> [&Entity<Region>; 4] {
        [&self.sidebar_header, &self.panel, &self.title_row, &self.status_bar]
    }

    /// Draw every region, and the list, again on the next frame. Called when the workspace was notified,
    /// and so only outside a draw.
    pub(super) fn notify(&self, cx: &mut App) {
        for region in self.all() {
            region.update(cx, |_, cx| cx.notify());
        }
        self.list.update(cx, |_, cx| cx.notify());
    }

    /// Draw the status bar again on the next frame and nothing else, as an animation in it asks to.
    #[cfg(test)]
    pub(super) fn notify_status_bar(&self, cx: &mut App) {
        self.status_bar.update(cx, |_, cx| cx.notify());
    }

    /// Draw the sidebar's header again on the next frame (outside a draw).
    fn notify_header(&self, cx: &mut App) {
        self.sidebar_header.update(cx, |_, cx| cx.notify());
    }

    /// The sidebar's header, across the top of its column.
    pub(super) fn sidebar_header(&self) -> AnyElement {
        let style = StyleRefinement::default().w_full().h(px(crate::style::TITLE_ROW_H)).flex_none();
        self.embed(Part::SidebarHeader, style)
    }

    /// The terminal list under the header: all the column's height that is left.
    ///
    /// Not a [`Region`]: it is drawn from the sidebar entity itself, so a notification of the list
    /// (a hover, a rename being typed, a turn of an agent's spinner) draws it and the views above it in the
    /// tree, and the header, which is a sibling and not a parent of it, is left alone.
    pub(super) fn sidebar_list(&self) -> AnyElement {
        let style = StyleRefinement::default().flex_1().min_h_0().w_full();
        crate::window::cached::view(&self.list, style)
    }

    /// The right panel's column, `width` wide and as tall as the window.
    pub(super) fn panel(&self, width: f32) -> AnyElement {
        let style = StyleRefinement::default().w(px(width)).h_full().flex_none();
        self.embed(Part::Panel, style)
    }

    /// The terminal's title row, across the top of its column.
    pub(super) fn title_row(&self) -> AnyElement {
        let style =
            StyleRefinement::default().absolute().top_0().left_0().right_0().h(px(crate::style::TITLE_ROW_H));
        self.embed(Part::TitleRow, style)
    }

    /// The status bar, across the bottom of the window.
    pub(super) fn status_bar(&self) -> AnyElement {
        let style = StyleRefinement::default().w_full().h(px(STATUS_BAR_H)).flex_none();
        self.embed(Part::StatusBar, style)
    }

    /// Tell the regions when a global they draw from changes. The workspace's own state notifies the
    /// workspace; a global changes without it, and the regions replay what they drew until told.
    ///
    /// The settings and the apps Open in… lists are watched whole. The shared state is not: it is marked
    /// changed by every write, and writes are made for reasons the chrome does not show, so the window
    /// compares the facts it draws from it ([`AppFacts`]) and notifies when one differs.
    pub(super) fn watch_globals(workspace: &mut Workspace, cx: &mut Context<Workspace>) {
        watch::<crate::settings::SettingsStore>(cx);
        watch::<crate::open_in::OpenIn>(cx);
        workspace.regions.app_seen = Some(AppFacts::of(cx));
        cx.observe_global::<crate::app_state::AppState>(|workspace, cx| workspace.app_state_changed(cx))
            .detach();
    }

    /// Tell the regions when the list changes in a way they draw. The list notifies for its own drawing (a
    /// hover, a rename being typed, a turn of an agent's spinner, 30 times a second). GPUI marks a
    /// notified view and its ancestors dirty, so the list is a sibling of the cached header in the column
    /// and not a child of it: those notifications draw the list and leave every region as it was. The
    /// regions read two facts of the list's, which are heard here: the saved view's tint of the selected
    /// terminal's group (the title row) and the profile chip's menu being open (the header, which draws
    /// the chip). The tab bar, which the root draws from the list's state when the terminals are in tabs, is
    /// the one other reader. It is not a region: the root reads the list while it builds the bar, so GPUI
    /// draws the root again for every notification of the list then, and the bar with it, without a word
    /// from here.
    pub(super) fn watch_sidebar(workspace: &mut Workspace, cx: &mut Context<Workspace>) {
        workspace.regions.paint_seen = workspace.group_paint(cx);
        let sidebar = workspace.sidebar.clone();
        workspace.regions.menu_seen = sidebar.read(cx).profile_page();
        cx.observe(&sidebar, |workspace, sidebar, cx| {
            let paint = workspace.group_paint(cx);
            let moved = std::mem::replace(&mut workspace.regions.paint_seen, paint) != paint;
            if moved {
                cx.notify();
            }
            let menu = sidebar.read(cx).profile_page();
            if std::mem::replace(&mut workspace.regions.menu_seen, menu) != menu {
                workspace.regions.notify_header(cx);
            }
        })
        .detach();
    }

    /// How many times each region has been built, in the order sidebar header, panel, title row, status bar.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn builds(&self, cx: &App) -> [usize; 4] {
        self.all().map(|region| region.read(cx).builds)
    }

    /// The root is being built: a frame begins (nothing may notify the workspace until it ends).
    #[cfg(any(test, debug_assertions))]
    pub(super) fn begin_frame(&self) {
        self.oracle.begin_frame();
    }

    /// `what` notifies the workspace: not while it draws.
    #[cfg(any(test, debug_assertions))]
    pub(super) fn assert_not_drawing(&self, what: &str) {
        self.oracle.assert_not_drawing(what);
    }

    fn entity(&self, part: Part) -> &Entity<Region> {
        match part {
            Part::SidebarHeader => &self.sidebar_header,
            Part::Panel => &self.panel,
            Part::TitleRow => &self.title_row,
            Part::StatusBar => &self.status_bar,
        }
    }

    /// `part` laid out at `style`: replayed from the last frame while nothing in it changed (`window::cached`),
    /// with its accessibility nodes.
    fn embed(&self, part: Part, style: StyleRefinement) -> AnyElement {
        #[cfg(any(test, debug_assertions))]
        self.oracle.place(part);
        crate::window::cached::view(self.entity(part), style)
    }
}

/// The check on the chrome (`oracle`) is for debug and test builds; a release build has nothing to check.
#[cfg(not(any(test, debug_assertions)))]
impl Workspace {
    pub(in crate::window) fn check_regions(&mut self, _: &Window, _: &App) {}
}

/// Notify the regions when global `G` changes.
fn watch<G: Global>(cx: &mut Context<Workspace>) {
    cx.observe_global::<G>(|workspace, cx| workspace.regions.notify(cx)).detach();
}

impl Workspace {
    /// The shared state was written: draw the regions again if a fact they draw from it differs from the
    /// last one seen.
    ///
    /// GPUI starts a global observer a moment after it is made, and a window made in the update that
    /// writes the state (it is listed in `windows` after it was built) is not told of that write, so the
    /// window that lists it tells it (`open_workspace_window_in`). Nor does its own observer of itself
    /// run yet, which is why the regions are told here as well as the workspace.
    pub(in crate::window) fn app_state_changed(&mut self, cx: &mut Context<Self>) {
        let now = AppFacts::of(cx);
        if self.regions.app_seen.as_ref() != Some(&now) {
            self.regions.app_seen = Some(now);
            self.regions.notify(cx);
            cx.notify();
        }
    }

    /// Build `part` as the window's state stands: what a [`Region`] draws when it is built.
    fn render_part(&mut self, part: Part, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let element = match part {
            // The row fills the view's own bounds, which a root with no size of its own would not.
            Part::SidebarHeader => {
                div().w_full().child(self.render_sidebar_header(window, cx)).into_any_element()
            }
            Part::Panel => {
                let width = self.columns().panel;
                self.render_panel(width, window, cx).into_any_element()
            }
            // The row positions itself at the top of its column; here it is placed in the view's own
            // bounds, which a root with no size of its own would not fill.
            Part::TitleRow => {
                self.regions.drawn.crumb = self.crumb_drawn(cx);
                div()
                    .relative()
                    .size_full()
                    .child(self.render_terminal_title_row(window, cx))
                    .into_any_element()
            }
            Part::StatusBar => {
                // What the bar drew is what the tick compares the next one with, and the counts that
                // differ from the ones it drew before turn over.
                let was = self.regions.drawn.status_bar.take();
                self.regions.drawn.status_bar = self.status_bar(cx);
                self.note_rolls(was.as_ref(), cx);
                // A column of its own, as the window's was: the bar fills its width, and a view's
                // root is only as wide as its content unless something makes it.
                let bar =
                    self.regions.drawn.status_bar.clone().map(|bar| self.render_status_bar(&bar, window, cx));
                div().flex().flex_col().w_full().children(bar).into_any_element()
            }
        };
        #[cfg(any(test, debug_assertions))]
        self.record_region(part, window, cx);
        element
    }
}
