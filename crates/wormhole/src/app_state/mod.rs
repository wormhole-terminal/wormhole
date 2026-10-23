//! State shared by every window: the database, the profiles, the settings and Theme Studio windows, and
//! the list of open windows.
//!
//! Each window's [`Workspace`] owns only its own sessions. Anything that must
//! exist once (the menu bar item, the hook socket, the updater) lives here and
//! is driven by one app-level tick and two message tasks (`app::launch`, `app::tick`), so it works
//! the same with one window or five; windows open and close in `window_lifecycle`.
//!
//! One tier with the window (`window`): the shared state names the window's type and opens and
//! closes windows through it, and a window reads the shared state. The status item's `MenuState` and
//! the updater's `Updates` are globals of their own, set at launch.

pub(crate) mod attention;
pub(crate) mod look;
pub(crate) mod profiles;
pub(crate) mod recent_projects;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::git::worktree::{self as worktree, Worktree, Worktrees};
use wormhole_core::onboarding::{DISMISSED_TIPS_KEY, DismissedTips, Tip};
use wormhole_core::storage::paths::{DataRoot, ProfileId};
use wormhole_core::storage::store::Store;
use wormhole_core::view::layout::Panes;
use wormhole_core::view::layout::{REMEMBERED_PANES_KEY, RememberedPanes};

use crate::app_state::look::LookOverride;
use crate::app_state::recent_projects::RecentProjects;
use crate::window::Workspace;

/// A secondary window that exists at most once: its handle, and its view (for devctl).
pub(crate) type Slot = Option<(AnyWindowHandle, AnyWeakEntity)>;

/// The window and view in `slot` if the view is still alive: `None` for an empty slot and for a window
/// that has closed (its weak handle no longer upgrades).
pub(crate) fn live(slot: &Slot) -> Option<(AnyWindowHandle, AnyEntity)> {
    let (handle, view) = slot.as_ref()?;
    Some((*handle, view.upgrade()?))
}

/// A window showing a [`Workspace`].
#[derive(Clone)]
pub(crate) struct OpenWindow {
    pub id: i64,
    pub handle: AnyWindowHandle,
    pub workspace: WeakEntity<Workspace>,
    /// The profile whose window this is.
    pub profile: ProfileId,
    /// Out of sight because its profile is not the shown one; its terminals run on.
    pub hidden: bool,
}

pub(crate) struct AppState {
    /// The data folder as a whole: `profiles.json`, the update schedule and the other shared files.
    pub root: DataRoot,
    /// Every profile, and the one Wormhole is in.
    pub profiles: ProfileList,
    /// Why `profiles.json` was set aside and rebuilt at launch, for Settings to say (the Profile page).
    pub profiles_error: Option<String>,
    /// The profiles that were shown before and are not now (see `profiles`).
    parked: profiles::Parked,
    pub store: Option<Store>,
    /// The tips closed with "Don't show again", kept in the store across launches.
    pub tips: DismissedTips,
    /// The projects worked in lately (Open Recent), most recent first.
    pub recent_projects: RecentProjects,
    /// The settings window, while open, and its page: untyped (the page is `settings_window::SettingsWindow`,
    /// which sits above the shared state and reads it back typed with `current_window`). Weak, like the
    /// Studio's: closing the window drops the page, and the page's release (the global hotkey comes back
    /// if its field was recording) must run then, not when the window is opened again.
    pub settings_window: Slot,
    /// The Theme Studio window, while open. Weak, so closing the window drops
    /// the Studio (which clears its preview); the Studio module reads it typed.
    pub studio_window: Slot,
    /// The About and Licenses windows, while open.
    pub about_window: Slot,
    pub licenses_window: Slot,
    /// Open windows, oldest first.
    pub windows: Vec<OpenWindow>,
    /// The window that was in front last: its session's project settings apply.
    pub front: i64,
    /// Set once the app is going away, so closing windows then never deletes sessions.
    pub quitting: bool,
    /// Quit and End All Terminals is ending every session host: the saves on the way out record no
    /// host for any terminal (`hosts::persisted_host`), so they come back as text.
    pub ending_hosts: bool,
    /// The session hosts found at launch, until the terminals that name them have attached and the
    /// orphans have been adopted (`hosting`).
    pub launch_hosts: wormhole_core::ipc::hosts::LaunchHosts,
    /// The window showing the question before quitting, while it is up.
    pub quit_prompt: Option<i64>,
    /// What `gh` said about each branch's pull request, asked rarely (the title row's PR link).
    pub pr: wormhole_core::git::pr::PrCache,
    /// The git worktrees Wormhole made (Shell → New Terminal in New Worktree), kept across
    /// launches: the only ones closing a terminal may offer to remove.
    pub worktrees: Worktrees,
    /// The side columns as they were last left, in any window: how a window opened with no
    /// layout of its own starts (`open_workspace_window`). Kept in the store's `meta`.
    pub last_panes: RememberedPanes,
    /// Terminals on their way to another window (a group moved there): the new window takes the
    /// running terminal of a session it is opened with instead of starting a shell.
    pub adopted: HashMap<i64, Entity<crate::terminal_view::TerminalView>>,
    next_id: i64,
    next_window_id: i64,
    /// Settings revision whose theme was last pushed to GPUI Kit.
    applied_theme: Option<u64>,
    /// How many app ticks there have been. A cell: counting one is no change anyone could read, and
    /// writing the global for it would mark the state changed every 400 ms.
    ticks: std::cell::Cell<u64>,
    /// What devctl says in place of the system (`appearance`, `reduce-transparency`,
    /// `increase-contrast`, `reduce-motion`).
    look_override: LookOverride,
    /// Hears the accessibility display options change while the app runs.
    _display_options: Option<crate::platform::look::DisplayObserver>,
}

impl Global for AppState {}

impl AppState {
    /// The state at launch: the shown profile's data, the profiles, the menu bar item and the session
    /// hosts found. The id counters start past every profile's database, so an id never repeats
    /// across profiles.
    pub(crate) fn new(
        root: DataRoot,
        profiles: ProfileList,
        profiles_error: Option<String>,
        data: profiles::ProfileData,
        launch_hosts: wormhole_core::ipc::hosts::LaunchHosts,
        display_options: Option<crate::platform::look::DisplayObserver>,
    ) -> Self {
        let profiles::ProfileData { store, tips, recent, worktrees, last_panes } = data;
        let (next_id, next_window_id) = profiles::first_free_ids(&root, &profiles, store.as_ref());
        Self {
            root,
            profiles,
            profiles_error,
            parked: Default::default(),
            store,
            tips,
            recent_projects: recent,
            settings_window: None,
            studio_window: None,
            about_window: None,
            licenses_window: None,
            windows: Vec::new(),
            front: 0,
            quitting: false,
            ending_hosts: false,
            launch_hosts,
            quit_prompt: None,
            pr: Default::default(),
            worktrees,
            last_panes,
            adopted: HashMap::new(),
            next_id,
            next_window_id,
            applied_theme: None,
            ticks: std::cell::Cell::new(0),
            look_override: LookOverride::default(),
            _display_options: display_options,
        }
    }

    /// The first id nobody has been given yet.
    pub(crate) fn next_id(&self) -> i64 {
        self.next_id
    }

    /// Count one app tick (400 ms) and say which one it is.
    pub(crate) fn advance_tick(&self) -> u64 {
        self.ticks.set(self.ticks.get() + 1);
        self.ticks.get()
    }
}

#[cfg(test)]
impl AppState {
    /// An app with no database and no windows, for tests that open a `Workspace` themselves.
    pub(crate) fn for_test(next_id: i64) -> Self {
        Self {
            root: DataRoot::current(),
            profiles: ProfileList::default(),
            profiles_error: None,
            parked: Default::default(),
            store: None,
            tips: DismissedTips::default(),
            recent_projects: RecentProjects::default(),
            settings_window: None,
            studio_window: None,
            about_window: None,
            licenses_window: None,
            windows: Vec::new(),
            front: 0,
            quitting: false,
            ending_hosts: false,
            launch_hosts: Default::default(),
            quit_prompt: None,
            pr: Default::default(),
            worktrees: Worktrees::default(),
            last_panes: RememberedPanes::default(),
            adopted: HashMap::new(),
            next_id,
            next_window_id: 2,
            applied_theme: None,
            ticks: std::cell::Cell::new(0),
            look_override: LookOverride::default(),
            _display_options: None,
        }
    }
}

pub(crate) fn app(cx: &App) -> &AppState {
    cx.global::<AppState>()
}

pub(crate) fn app_mut(cx: &mut App) -> &mut AppState {
    cx.global_mut::<AppState>()
}

impl AppState {
    /// "Don't show again" on a tip: closed for good, remembered across launches.
    pub(crate) fn dismiss_tip(&mut self, tip: Tip) {
        if self.tips.dismiss(tip) {
            self.save_tips();
        }
    }

    /// Settings → Reset tips: every tip closed for good shows again where it belongs.
    pub(crate) fn reset_tips(&mut self) {
        if !self.tips.is_empty() {
            self.tips.clear();
            self.save_tips();
        }
    }

    /// `root`, the project of the terminal in front, counts as recently used (Open Recent, the status
    /// item's menu).
    pub(crate) fn remember_project(&mut self, root: PathBuf) {
        self.recent_projects.remember(root, self.store.as_ref());
    }

    /// Whether `root` already leads the recent projects: remembering it would change nothing. Asked
    /// first by whatever remembers the project in front on every tick, since writing the state marks it
    /// changed.
    pub(crate) fn leads_recent_projects(&self, root: &Path) -> bool {
        self.recent_projects.roots().first().is_some_and(|first| first == root)
    }

    fn save_tips(&self) {
        if let Some(store) = &self.store
            && let Err(err) = store.set_meta(DISMISSED_TIPS_KEY, &self.tips.to_json())
        {
            tracing::warn!("remembering the closed tips: {err:#}");
        }
    }

    /// A worktree was just made: remembered, so closing its terminal can offer to remove it.
    pub(crate) fn record_worktree(&mut self, made: Worktree) {
        self.worktrees.record(made);
        self.save_worktrees();
    }

    /// The worktree at `folder` was removed (or is the user's to keep): forgotten.
    pub(crate) fn forget_worktree(&mut self, folder: &Path) {
        if self.worktrees.forget(folder) {
            self.save_worktrees();
        }
    }

    /// A column was opened, closed or resized in some window: that is how new windows start
    /// from now on. Written only when it differs, so the callers needn't be sparing.
    pub(crate) fn remember_panes(&mut self, panes: &Panes) {
        let now = RememberedPanes::of(panes);
        if now == self.last_panes {
            return;
        }
        self.last_panes = now;
        if let Some(store) = &self.store
            && let Err(err) = store.set_meta(REMEMBERED_PANES_KEY, &now.to_json())
        {
            tracing::warn!("remembering the side columns: {err:#}");
        }
    }

    fn save_worktrees(&self) {
        if let Some(store) = &self.store
            && let Err(err) = store.set_meta(worktree::STORE_KEY, &self.worktrees.to_json())
        {
            tracing::warn!("remembering the worktrees: {err:#}");
        }
    }
}

impl AppState {
    pub(crate) fn alloc_window_id(&mut self) -> i64 {
        let id = self.next_window_id;
        self.next_window_id += 1;
        id
    }

    /// The window the user is working in: the active one, else the one that
    /// was in front last.
    pub(crate) fn front_window(cx: &App) -> Option<OpenWindow> {
        let state = app(cx);
        // A window of a profile that is not shown is never the one the person works in.
        let mut shown = state.windows.iter().filter(|w| !w.hidden);
        cx.active_window()
            .and_then(|active| shown.clone().find(|w| w.handle.window_id() == active.window_id()))
            .or_else(|| shown.clone().find(|w| w.id == state.front))
            .or_else(|| shown.next())
            .cloned()
    }

    /// The window whose workspace satisfies `pred`.
    pub(crate) fn find_window(
        cx: &App,
        mut pred: impl FnMut(&Workspace, &App) -> bool,
    ) -> Option<OpenWindow> {
        app(cx)
            .windows
            .iter()
            .find(|w| w.workspace.read_with(cx, |ws, cx| pred(ws, cx)).unwrap_or(false))
            .cloned()
    }

    /// Run `f` on a window's workspace. `None` when the window is gone or is
    /// being updated right now (so don't call this for the window you are in).
    pub(crate) fn with_workspace<R>(
        window: &OpenWindow,
        cx: &mut App,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
    ) -> Option<R> {
        window
            .handle
            .update(cx, |_, win, cx| window.workspace.update(cx, |ws, cx| f(ws, win, cx)).ok())
            .ok()
            .flatten()
    }
}
