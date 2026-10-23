//! The window's chrome as views GPUI keeps between frames: the sidebar's column, the right panel,
//! the terminal's title row and the status bar.
//!
//! GPUI draws the window's root again for every frame a terminal streams, and with it every div the
//! chrome is made of, although a terminal's output changes none of it. A [`Region`] is a view that draws
//! one part of the chrome through the workspace's own builders, and is laid out `cached`: a frame that
//! changes nothing in it replays what it drew last time (bounds, content mask and text style
//! unchanged, GPUI's rule), without building, laying out or painting it again.
//!
//! What makes it draw again is the workspace being notified, or one of the globals the parts read
//! changing ([`Regions::watch_globals`]). Both reach the regions outside a draw, which is the only
//! time GPUI acts on a notification for a view it replays. The facts the parts show reach the
//! workspace already: a terminal's summary changing, a hook, the tick comparing the words that move
//! with time (`refresh_status_bar`, `crumb_tick`, the sidebar's `times_changed`), a feature's entity
//! changing (`feature_host`) and the settings. A new fact a part draws must notify the workspace the
//! same way; one that only changes in a draw would be missed.
//!
//! While an assistive app is connected the regions are not cached at all (`embed`): GPUI rebuilds the
//! accessibility tree in every frame and replays none of a reused view's nodes, so a cached region would
//! drop out of the tree (and its buttons stop answering VoiceOver) at the terminal's frame rate.
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

use gpui_kit::*;
use wormhole_core::view::layout::status_bar::STATUS_BAR_H;

use crate::window::Workspace;
use crate::window::status_bar::StatusBar;

/// A part of the chrome that is a view of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// The sidebar's column: its header over the terminal list and the first-run note.
    Sidebar,
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
    const ALL: [Part; Self::COUNT] = [Part::Sidebar, Part::Panel, Part::TitleRow, Part::StatusBar];

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
    sidebar: Entity<Region>,
    panel: Entity<Region>,
    title_row: Entity<Region>,
    status_bar: Entity<Region>,
    /// What the regions last drew of what moves with time, to notice on the tick when it is stale.
    pub(super) drawn: Drawn,
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
    pub(super) crumb: std::cell::RefCell<Option<String>>,
}

impl Regions {
    pub(super) fn new(workspace: WeakEntity<Workspace>, cx: &mut App) -> Self {
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
            sidebar: region(Part::Sidebar),
            panel: region(Part::Panel),
            title_row: region(Part::TitleRow),
            status_bar: region(Part::StatusBar),
            drawn: Drawn::default(),
            #[cfg(any(test, debug_assertions))]
            oracle: Default::default(),
        }
    }

    fn all(&self) -> [&Entity<Region>; 4] {
        [&self.sidebar, &self.panel, &self.title_row, &self.status_bar]
    }

    /// Draw every region again on the next frame. Called when the workspace was notified, and so only
    /// outside a draw.
    pub(super) fn notify(&self, cx: &mut App) {
        for region in self.all() {
            region.update(cx, |_, cx| cx.notify());
        }
    }

    /// The sidebar's column, `width` wide and as tall as the window.
    pub(super) fn sidebar(&self, width: f32, window: &Window) -> AnyElement {
        let style = StyleRefinement::default().w(px(width)).h_full().flex_none();
        self.embed(Part::Sidebar, style, window)
    }

    /// The right panel's column, `width` wide and as tall as the window.
    pub(super) fn panel(&self, width: f32, window: &Window) -> AnyElement {
        let style = StyleRefinement::default().w(px(width)).h_full().flex_none();
        self.embed(Part::Panel, style, window)
    }

    /// The terminal's title row, across the top of its column.
    pub(super) fn title_row(&self, window: &Window) -> AnyElement {
        let style =
            StyleRefinement::default().absolute().top_0().left_0().right_0().h(px(crate::style::TITLE_ROW_H));
        self.embed(Part::TitleRow, style, window)
    }

    /// The status bar, across the bottom of the window.
    pub(super) fn status_bar(&self, window: &Window) -> AnyElement {
        let style = StyleRefinement::default().w_full().h(px(STATUS_BAR_H)).flex_none();
        self.embed(Part::StatusBar, style, window)
    }

    /// Tell the regions when a global they draw from changes. The workspace's own state notifies the
    /// workspace; a global changes without it, and the regions replay what they drew until told.
    pub(super) fn watch_globals(cx: &mut Context<Workspace>) {
        watch::<crate::settings::SettingsStore>(cx);
        watch::<crate::app_state::AppState>(cx);
        watch::<crate::open_in::OpenIn>(cx);
    }

    /// How many times each region has been built, in the order sidebar, panel, title row, status bar.
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
            Part::Sidebar => &self.sidebar,
            Part::Panel => &self.panel,
            Part::TitleRow => &self.title_row,
            Part::StatusBar => &self.status_bar,
        }
    }

    /// `part` laid out at `style`: replayed from the last frame while nothing in it changed, except while
    /// the accessibility tree is built. GPUI fills the tree during every frame's prepaint and replays no
    /// nodes, so a replayed region would vanish from it (VoiceOver would lose the sidebar's rows, the
    /// panel's tabs and the bar's buttons at the terminal's frame rate). Then the region is a plain child
    /// in a box of the same style, built in every frame like the rest of the window.
    fn embed(&self, part: Part, style: StyleRefinement, window: &Window) -> AnyElement {
        #[cfg(any(test, debug_assertions))]
        self.oracle.place(part);
        let region = self.entity(part).clone();
        if crate::platform::a11y::tree_built(window) {
            let mut shell = div();
            *shell.style() = style;
            shell.child(region).into_any_element()
        } else {
            region.cached(style).into_any_element()
        }
    }
}

/// Notify the regions when global `G` changes.
fn watch<G: Global>(cx: &mut Context<Workspace>) {
    cx.observe_global::<G>(|workspace, cx| workspace.regions.notify(cx)).detach();
}

impl Workspace {
    /// Build `part` as the window's state stands: what a [`Region`] draws when it is built.
    fn render_part(&mut self, part: Part, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let element = match part {
            Part::Sidebar => {
                let width = self.columns().sidebar;
                self.render_sidebar(width, window, cx)
            }
            Part::Panel => {
                let width = self.columns().panel;
                self.render_panel(width, window, cx).into_any_element()
            }
            // The row positions itself at the top of its column; here it is placed in the view's own
            // bounds, which a root with no size of its own would not fill.
            Part::TitleRow => div()
                .relative()
                .size_full()
                .child(self.render_terminal_title_row(window, cx))
                .into_any_element(),
            Part::StatusBar => {
                // What the bar drew is what the tick compares the next one with.
                self.regions.drawn.status_bar = self.status_bar(cx);
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
