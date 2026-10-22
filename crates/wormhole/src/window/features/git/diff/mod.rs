//! The diff view: a file's changes between two revisions (or the working
//! tree), side by side or unified, with syntax highlighting, in a card that
//! replaces the terminal. `rows` builds the display rows (with `highlight`),
//! `load` fetches and rebuilds them, `render` and `header` draw the card and
//! `notes` what it says in place of lines; this file holds the state and what
//! opens, steps and closes it.

mod header;
mod highlight;
mod load;
mod notes;
pub(crate) mod render;
mod review;
mod rows;
mod submodule;

use std::cell::Cell as StdCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::SettingsStore;
use crate::settings::{settings_in, store_in, ui_in};
use crate::workspace::Workspace;
use crate::workspace::features::git::changes::display_order;
use crate::workspace::features::git::job::Job;
use crate::workspace::features::git::open_file::{OpenFile, OpenTarget};
use load::{Ask, Stamp};
use rows::{BuildKey, Row, RowStats};
use wormhole_core::git::{self, DiffLimits, DiffSpec, FileChange, FileDiff, Side, SubmoduleLog};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::limits;

pub(super) use load::Observed;
pub(in crate::window) use review::Commenting;

/// What a diff shows: one file between two sides of a repository.
#[derive(Clone, Debug, PartialEq)]
struct Request {
    root: PathBuf,
    file: FileChange,
    spec: DiffSpec,
    /// [`DiffLimits::LOAD_ANYWAY`] once the user chose to load a diff too large to show
    /// unasked; it holds for the refreshes of that file.
    limits: DiffLimits,
}

pub struct DiffView {
    /// The profile of the window showing it: whose settings and colours it draws with.
    profile: ProfileId,
    request: Request,
    /// Files ‹ › steps through (the Changes list in the order it shows them, or a commit's files).
    siblings: Rc<Vec<FileChange>>,
    /// Shown before the path, e.g. the commit ("a1b2c3d").
    context: Option<SharedString>,
    /// What is being fetched or built: the first load, a refresh after an edit,
    /// or the rows for new settings. A result is applied only if it answers this.
    job: Job<Ask>,
    /// Nothing to show yet. A refresh keeps the old rows (and the scroll position) up.
    loading: bool,
    error: Option<SharedString>,
    /// Why the last refresh failed; the diff on screen is from before it.
    refresh_error: Option<SharedString>,
    diff: Option<Arc<FileDiff>>,
    /// For a submodule: the commits between its two sides, in place of a diff.
    submodule: Option<Arc<SubmoduleLog>>,
    /// The working-tree file as the diff was read, to notice that it was edited since.
    stamp: Option<Stamp>,
    /// The file was in the working tree when the diff was read: Reveal in Finder has something
    /// to show, also for a commit's file.
    on_disk: bool,
    /// The file is no longer among the changes (reverted, committed); the diff is what it was.
    gone: bool,
    /// What the old and new side hold (text, nothing, binary).
    sides: (Side, Side),
    /// Lines added / removed, counted from the diff itself.
    counts: (usize, usize),
    rows: Rc<Vec<Row>>,
    stats: RowStats,
    built: Option<BuildKey>,
    /// The syntax fingerprint of the settings revision it was last computed for.
    syntax_seen: Option<(u64, u64)>,
    scroll: UniformListScrollHandle,
    /// Variable-height list used when wrapping.
    wrap_list: ListState,
    /// The code font size the wrap list's row heights were measured at.
    measured_font: StdCell<f32>,
    /// How far the code is scrolled sideways; the gutters stay put.
    scroll_x: f32,
    /// The farthest `scroll_x` can go, from the last frame's width.
    max_scroll_x: StdCell<f32>,
    /// The axis the trackpad gesture in progress keeps to.
    gesture: OngoingScroll,
    /// Width of the rows' area in the last frame.
    body_width: Rc<StdCell<f32>>,
}

impl DiffView {
    pub fn set_profile(&mut self, profile: ProfileId) {
        self.profile = profile;
    }

    fn new(request: Request, siblings: Rc<Vec<FileChange>>, context: Option<SharedString>) -> Self {
        let file = &request.file;
        Self {
            profile: ProfileId::default_profile(),
            sides: git::sides(&FileDiff::default(), file.kind),
            counts: load::file_counts(file),
            request,
            siblings,
            context,
            job: Job::default(),
            loading: true,
            error: None,
            refresh_error: None,
            diff: None,
            submodule: None,
            stamp: None,
            on_disk: false,
            gone: false,
            rows: Rc::default(),
            stats: RowStats::default(),
            built: None,
            syntax_seen: None,
            scroll: UniformListScrollHandle::new(),
            wrap_list: ListState::new(0, ListAlignment::Top, px(400.0)),
            measured_font: StdCell::new(0.0),
            scroll_x: 0.0,
            max_scroll_x: StdCell::new(0.0),
            gesture: OngoingScroll::default(),
            body_width: Rc::default(),
        }
    }

    /// The file shown, as the last status described it.
    pub(super) fn file(&self) -> &FileChange {
        &self.request.file
    }

    /// The diff was too large to load without asking.
    fn too_large(&self) -> bool {
        self.diff.as_ref().is_some_and(|d| d.too_large)
    }

    /// Whether the rows on hand are side by side (they may lag the setting, or the room for
    /// it, `split_fits`, by a frame).
    fn split(&self, split_fits: bool, cx: &App) -> bool {
        self.built.map_or(split_fits && settings_in(&self.profile, cx).diff_split, |k| k.split)
    }

    /// Stored with Git LFS: each side a pointer, or `.gitattributes` says so.
    fn lfs(&self) -> bool {
        self.request.file.lfs || self.diff.as_ref().is_some_and(|d| d.lfs.is_some())
    }

    /// How far a wheel or trackpad event moves the code sideways. A trackpad gesture keeps to
    /// the axis it started on, by the same rule the list follows for its rows, so the view
    /// never scrolls diagonally.
    fn sideways(&mut self, mut delta: Point<Pixels>, phase: TouchPhase, precise: bool) -> f32 {
        if precise {
            self.gesture.filter(&mut delta, phase);
        }
        f32::from(delta.x)
    }

    /// The file on disk that Reveal in Finder shows: the diff's path, whenever a file is
    /// there now, whatever revisions the diff compares. `None` for a file that is deleted, or
    /// gone since the commit.
    fn on_disk(&self) -> Option<PathBuf> {
        self.on_disk.then(|| self.request.root.join(&self.request.file.path))
    }

    /// The file as the icon that opens it in the editor names it: its path in the repository,
    /// when it is in the working tree (a deleted file has nothing to open).
    fn open_file(&self, cx: &App) -> Option<OpenFile> {
        self.on_disk.then(|| OpenTarget::new(&self.request.root, cx).file(&self.request.file.path))
    }

    /// What the Reveal button says: in History and compare the file on disk may not be what the
    /// diff shows (its new side is a commit, or its old one a branch point), so the button says
    /// which file it reveals; for the working tree against HEAD it is the one on view.
    fn reveal_label(&self) -> &'static str {
        let against_head = self.request.spec.old == "HEAD" || self.request.spec.old == git::EMPTY_TREE;
        if self.is_working_tree() && against_head {
            "Reveal in Finder"
        } else {
            "Reveal Current File in Finder"
        }
    }
}

impl Workspace {
    // ------------------------------------------------------------ opening

    /// Open a working-tree change from the Changes tab.
    pub(crate) fn open_change(&mut self, file: FileChange, cx: &mut Context<Self>) {
        let Some(status) = &self.active_session().git.status else { return };
        let spec = DiffSpec { old: status.base_rev.clone(), new: None };
        let s = settings_in(&self.profile, cx);
        let siblings = Rc::new(display_order(status, s.changes_tree, s.group_untracked));
        let root = status.root.clone();
        self.open_diff(root, file, spec, siblings, None, cx);
    }

    /// Show `file` between the sides `spec` names. Replaces the diff on view; the
    /// old one's work, still running, finds nobody waiting and stops.
    pub(crate) fn open_diff(
        &mut self,
        root: PathBuf,
        file: FileChange,
        spec: DiffSpec,
        siblings: Rc<Vec<FileChange>>,
        context: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let request = Request { root, file, spec, limits: DiffLimits::default() };
        let mut view = DiffView::new(request, siblings, context);
        view.profile = self.profile.clone();
        let split_fits = self.diff_split_fits(cx);
        let store = store_in(&self.profile, cx);
        let build = BuildKey::of(&store.settings, split_fits, view.syntax(store.revision, &store.tokens));
        let started = view.begin_load(build);
        self.diff_view = Some(view);
        if let Some((wanted, work)) = started {
            self.spawn_diff_work(wanted, work, build, cx);
        }
        cx.notify();
    }

    /// The user chose to load a diff that was too large to load unasked.
    pub(crate) fn load_diff_anyway(&mut self, cx: &mut Context<Self>) {
        let Some(build) = self.build_key(cx) else { return };
        let Some(view) = self.diff_view.as_mut() else { return };
        if let Some((wanted, work)) = view.begin_load_anyway(build) {
            self.spawn_diff_work(wanted, work, build, cx);
            cx.notify();
        }
    }

    /// Whether the diff has room for old and new side by side: its body is the terminal's
    /// column less the card's inset and border, and needs two columns of 280.
    pub(super) fn diff_split_fits(&self, cx: &App) -> bool {
        let border = f32::from(ui_in(&self.profile, cx).border_width);
        limits::diff_split_fits(self.columns().terminal - 2.0 * (render::CARD_INSET + border))
    }

    /// Show the open diff's file in the Finder (the card of a diff too large to load).
    // GPUI's test platform panics on `reveal_path` (gpui-pre 0.3.9, `platform/test/platform.rs`), so
    // a view test that clicks this needs a stand-in first.
    pub(crate) fn reveal_diff_file(&mut self, cx: &App) {
        let Some(path) = self.diff_view.as_ref().and_then(DiffView::on_disk) else { return };
        cx.reveal_path(&path);
    }

    pub(crate) fn close_diff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.diff_view.take().is_some() {
            window.focus(&self.active_terminal().focus_handle(cx), cx);
            cx.notify();
        }
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn show_changes(&mut self, cx: &mut Context<Self>) {
        self.show_tab(crate::workspace::RightTab::Changes, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn open_nth_diff(&mut self, ix: usize, cx: &mut Context<Self>) {
        let file = self.active_session().git.status.as_ref().and_then(|s| s.files.get(ix).cloned());
        if let Some(file) = file {
            self.open_change(file, cx);
        }
    }

    /// Side by side or unified; remembered in config.json.
    pub(crate) fn set_diff_split(&mut self, split: bool, cx: &mut Context<Self>) {
        SettingsStore::update(cx, |s| s.diff_split = split);
        self.sync_diff(cx);
        cx.notify();
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn toggle_split(&mut self, cx: &mut Context<Self>) {
        let split = settings_in(&self.profile, cx).diff_split;
        self.set_diff_split(!split, cx);
    }

    /// Open the previous / next changed file.
    pub(crate) fn step_diff(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(view) = &self.diff_view else { return };
        let files = view.siblings.clone();
        let Some(ix) = files.iter().position(|f| f.path == view.file().path) else { return };
        let next = (ix as isize + delta).rem_euclid(files.len() as isize) as usize;
        let (root, spec, context) =
            (view.request.root.clone(), view.request.spec.clone(), view.context.clone());
        let file = files[next].clone();
        self.open_diff(root, file, spec, files, context, cx);
    }

    /// Sideways scrolling moves the code only, so line numbers stay in view.
    fn scroll_diff_sideways(&mut self, event: &ScrollWheelEvent, window: &Window, cx: &mut Context<Self>) {
        let Some(view) = self.diff_view.as_mut() else { return };
        let delta = event.delta.pixel_delta(window.line_height());
        let delta = view.sideways(delta, event.touch_phase, event.delta.precise());
        self.scroll_diff_by(delta, cx);
    }

    /// Scroll the code sideways by a wheel delta (positive = toward the start).
    pub(crate) fn scroll_diff_by(&mut self, delta: f32, cx: &mut Context<Self>) {
        let wrap = settings_in(&self.profile, cx).diff_wrap;
        let Some(view) = self.diff_view.as_mut() else { return };
        if delta == 0.0 || wrap {
            return;
        }
        let next = (view.scroll_x - delta).clamp(0.0, view.max_scroll_x.get());
        if next != view.scroll_x {
            view.scroll_x = next;
            cx.notify();
        }
    }
}
