//! State shared by every window: the database, project folders, menu bar,
//! updater, the settings and Theme Studio windows, and the list of open
//! windows with what happens as they open and close.
//!
//! Each window's [`Workspace`] owns only its own sessions. Anything that must
//! exist once (the menu bar item, the hook socket, the updater) lives here and
//! is driven by one app-level tick and two message tasks, so it works the same
//! with one window or five.
//!
//! What the system says about how windows look is heard here too: its appearance (for Match
//! system), through every window's appearance observer, and the accessibility display options
//! (Reduce Transparency, Increase Contrast, Reduce Motion), through their change notification
//! and again whenever a window comes to the front (macOS is not relied on to tell). The
//! look reaches the settings store, which re-derives what it touches; Reduce Motion is GPUI's
//! own flag, which its animations follow.

use wormhole_core::git::worktree::{self as worktree, Worktree, Worktrees};
use wormhole_core::layout::{REMEMBERED_PANES_KEY, RememberedPanes};
use wormhole_core::onboarding::{DISMISSED_TIPS_KEY, DismissedTips, Tip};
use wormhole_core::store::{Store, WindowRecord};
use wormhole_core::tokens::Appearance;

use super::*;
use crate::menubar::MenuCommand;
use crate::settings::SystemLook;

const TICK: Duration = Duration::from_millis(400);
const DEFAULT_SIZE: (f32, f32) = (1320.0, 820.0);

/// A window showing a [`Workspace`].
#[derive(Clone)]
pub(crate) struct OpenWindow {
    pub id: i64,
    pub handle: AnyWindowHandle,
    pub workspace: WeakEntity<Workspace>,
}

pub(crate) struct AppState {
    pub store: Option<Store>,
    /// The tips closed with "Don't show again", kept in the store across launches.
    pub tips: DismissedTips,
    /// `.wormhole/` project folders: preferences, trust and loaded files.
    pub hub: project::ProjectHub,
    /// Menu bar item and global hotkey.
    pub menu: menu_bar::MenuState,
    pub updates: updates::Updates,
    /// The settings window, while open.
    pub settings_window: Option<(AnyWindowHandle, Entity<settings_page::SettingsWindow>)>,
    /// The Theme Studio window, while open. Weak, so closing the window drops
    /// the Studio (which clears its preview).
    pub studio_window: Option<(AnyWindowHandle, WeakEntity<studio::StudioWindow>)>,
    /// The About and Licenses windows, while open.
    pub about_window: aux_window::Slot,
    pub licenses_window: aux_window::Slot,
    /// The window listing everything a project wants to run (the trust notice's "View all"),
    /// while open.
    pub trust_window: aux_window::Slot,
    /// Open windows, oldest first.
    pub windows: Vec<OpenWindow>,
    /// The window that was in front last: its session's project settings apply.
    pub front: i64,
    /// Set once the app is going away, so closing windows then never deletes sessions.
    pub quitting: bool,
    /// The window showing the question before quitting, while it is up.
    pub quit_prompt: Option<i64>,
    /// The coding agents' plan usage limits, one report per agent (the Usage tab).
    pub usage: Vec<wormhole_core::usage::Usage>,
    /// What `gh` said about each branch's pull request, asked rarely (the title row's PR link).
    pub pr: wormhole_core::git::pr::PrCache,
    /// The git worktrees Wormhole made (Shell → New Terminal in New Worktree), kept across
    /// launches: the only ones closing a terminal may offer to remove.
    pub worktrees: Worktrees,
    /// The side columns as they were last left, in any window: how a window opened with no
    /// layout of its own starts (`open_workspace_window`). Kept in the store's `meta`.
    pub last_panes: RememberedPanes,
    /// The terminal welcomes running, and the doors to them (core's token rules).
    pub welcomes: super::welcome::Welcomes,
    /// The first launch's window starts with a welcome terminal, not a plain one: set for as long as
    /// that window is being opened.
    pub first_welcome: Option<super::welcome::Plan>,
    next_id: i64,
    next_window_id: i64,
    /// Settings revision whose theme was last pushed to GPUI Kit.
    applied_theme: Option<u64>,
    ticks: u64,
    /// What devctl says in place of the system (`appearance`, `reduce-transparency`,
    /// `increase-contrast`, `reduce-motion`).
    look_override: LookOverride,
    /// Hears the accessibility display options change while the app runs.
    _display_options: Option<vibrancy::DisplayObserver>,
}

/// Stand-ins for what the system says about how windows look; `None` follows the system.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LookOverride {
    pub appearance: Option<Appearance>,
    pub reduce_transparency: Option<bool>,
    pub increase_contrast: Option<bool>,
    pub reduce_motion: Option<bool>,
}

impl Global for AppState {}

#[cfg(test)]
impl AppState {
    /// An app with no database and no windows, for tests that open a `Workspace` themselves.
    pub(super) fn for_test(next_id: i64) -> Self {
        let (menu, _) = menu_bar::MenuState::new(None);
        Self {
            store: None,
            tips: DismissedTips::default(),
            hub: project::ProjectHub::load(),
            menu,
            updates: updates::Updates::new(None),
            settings_window: None,
            studio_window: None,
            about_window: None,
            licenses_window: None,
            trust_window: None,
            windows: Vec::new(),
            front: 0,
            quitting: false,
            quit_prompt: None,
            usage: Vec::new(),
            pr: Default::default(),
            worktrees: Worktrees::default(),
            last_panes: RememberedPanes::default(),
            welcomes: Default::default(),
            first_welcome: None,
            next_id,
            next_window_id: 2,
            applied_theme: None,
            ticks: 0,
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

    /// An agent's latest plan usage limits, kept across restarts.
    fn record_usage(&mut self, usage: wormhole_core::usage::Usage) {
        wormhole_core::usage::record(&mut self.usage, usage);
        if let Some(store) = &self.store
            && let Err(err) =
                store.set_meta(wormhole_core::usage::STORE_KEY, &wormhole_core::usage::to_json(&self.usage))
        {
            tracing::warn!("remembering the usage limits: {err:#}");
        }
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
    /// Load what was saved, start the app-level tasks and open the windows
    /// that were open last time (or one fresh window).
    pub(crate) fn start(hook_events: smol::channel::Receiver<ipc::Incoming>, cx: &mut App) {
        upgrade_legacy_hooks(cx);
        cx.background_spawn(async { super::default_terminal::upgrade_legacy_default() }).detach();
        let store = Store::open().inspect_err(|err| tracing::error!("persistence disabled: {err:#}")).ok();
        let saved = store
            .as_ref()
            .and_then(|s| {
                s.load().inspect_err(|err| tracing::error!("loading saved sessions failed: {err:#}")).ok()
            })
            .unwrap_or_default();
        let (next_id, next_window_id) = store.as_ref().and_then(|s| s.next_ids().ok()).unwrap_or((1, 1));
        let tips = DismissedTips::parse(
            store.as_ref().and_then(|s| s.meta(DISMISSED_TIPS_KEY).ok().flatten()).as_deref(),
        );
        let usage = store
            .as_ref()
            .and_then(|s| s.meta(wormhole_core::usage::STORE_KEY).ok().flatten())
            .map(|json| wormhole_core::usage::from_json(&json))
            .unwrap_or_default();
        // Worktrees removed by hand since are forgotten, so nothing offers to remove them.
        let mut worktrees = store
            .as_ref()
            .and_then(|s| s.meta(worktree::STORE_KEY).ok().flatten())
            .map(|json| Worktrees::from_json(&json))
            .unwrap_or_default();
        worktrees.prune();
        let last_panes = RememberedPanes::parse(
            store.as_ref().and_then(|s| s.meta(REMEMBERED_PANES_KEY).ok().flatten()).as_deref(),
        );
        let (menu, menu_commands) = menu_bar::MenuState::new(store.as_ref());
        let updates = updates::Updates::new(store.as_ref());
        let mut hub = project::ProjectHub::load();
        hub.ids_start_at(next_id);
        let (display_changed, display_options) = smol::channel::unbounded::<()>();
        let display_observer = vibrancy::observe_display_options(move || {
            let _ = display_changed.try_send(());
        });
        super::open_in::OpenIn::install(cx);
        cx.set_global(AppState {
            store,
            tips,
            hub,
            menu,
            updates,
            settings_window: None,
            studio_window: None,
            about_window: None,
            licenses_window: None,
            trust_window: None,
            windows: Vec::new(),
            front: 0,
            quitting: false,
            quit_prompt: None,
            usage,
            pr: Default::default(),
            worktrees,
            last_panes,
            welcomes: Default::default(),
            first_welcome: None,
            next_id,
            next_window_id,
            applied_theme: None,
            ticks: 0,
            look_override: LookOverride::default(),
            _display_options: display_observer,
        });
        // Before the first frame, so windows don't open in the default colors (nor in the
        // dark pick when Match system and a light system ask for the light one).
        sync_system_look(cx);
        apply_theme(cx);
        cx.spawn(async move |cx| {
            while display_options.recv().await.is_ok() {
                cx.update(sync_system_look);
            }
        })
        .detach();

        // Every way of quitting (⌘Q, the menu bar, the Dock) ends up here.
        cx.on_app_quit(|cx| {
            save_all(cx);
            ipc::remove_socket();
            async {}
        })
        .detach();
        cx.on_window_closed(window_closed).detach();
        notifications::init(cx);

        cx.spawn(async move |cx| {
            while let Ok(message) = hook_events.recv().await {
                cx.update(|cx| dispatch_hook(message, cx));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            while let Ok(command) = menu_commands.recv().await {
                cx.update(|cx| handle_menu_command(command, cx));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                cx.update(tick);
            }
        })
        .detach();

        if saved.is_empty() {
            // With the terminal welcome on, the window's first terminal runs it and the run plays
            // around the window; otherwise the intro window opens over a plain terminal.
            let welcome = super::welcome::first_launch_plan(cx);
            app_mut(cx).first_welcome = welcome;
            open_workspace_window(None, None, cx);
            app_mut(cx).first_welcome = None;
            match welcome {
                Some(plan) => super::welcome::play_first(plan, cx),
                None => onboarding::start_if_first_launch(cx),
            }
        }
        for window in saved {
            open_workspace_window(Some(window), None, cx);
        }
        app_menu::sync(cx);
    }

    pub(crate) fn alloc_window_id(&mut self) -> i64 {
        let id = self.next_window_id;
        self.next_window_id += 1;
        id
    }

    /// Ids for sessions (terminals) and splits; hooks report a terminal's id, so they never repeat.
    pub(crate) fn alloc_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        // Belt and braces with `ids_start_at`: a fresh id never comes with a notes base.
        self.hub.id_allocated(id);
        id
    }

    pub(crate) fn save_window(&mut self, record: &WindowRecord) {
        if let Some(store) = &mut self.store
            && let Err(err) = store.save_window(record)
        {
            tracing::error!("saving window {} failed: {err:#}", record.id);
        }
    }

    /// A window was closed on purpose: its sessions go with it.
    pub(crate) fn forget_window(&mut self, id: i64) {
        if let Some(store) = &mut self.store
            && let Err(err) = store.delete_window(id)
        {
            tracing::error!("forgetting window {id} failed: {err:#}");
        }
    }

    /// The window the user is working in: the active one, else the one that
    /// was in front last.
    pub(crate) fn front_window(cx: &App) -> Option<OpenWindow> {
        let state = app(cx);
        cx.active_window()
            .and_then(|active| state.windows.iter().find(|w| w.handle.window_id() == active.window_id()))
            .or_else(|| state.windows.iter().find(|w| w.id == state.front))
            .or_else(|| state.windows.first())
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

    /// Run `f` on the front window's workspace, opening a window if there is none.
    pub(crate) fn with_front_workspace<R>(
        cx: &mut App,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
    ) -> Option<R> {
        match Self::front_window(cx) {
            Some(window) => Self::with_workspace(&window, cx, f),
            None => {
                open_workspace_window(None, None, cx);
                None
            }
        }
    }

    /// Projects with a session open in any window.
    pub(crate) fn open_project_roots(cx: &App) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for window in &app(cx).windows {
            if let Ok(found) = window.workspace.read_with(cx, |ws, cx| ws.project_roots(cx)) {
                roots.extend(found);
            }
        }
        roots
    }
}

/// Open a window showing `record` (a new, empty one without it), placed where
/// it was saved, or stepped away from `source`, or centered.
pub(crate) fn open_workspace_window(
    record: Option<WindowRecord>,
    source: Option<Bounds<Pixels>>,
    cx: &mut App,
) {
    let id = match &record {
        Some(record) => record.id,
        None => app_mut(cx).alloc_window_id(),
    };
    // No record: a window the person asked for with none open, so its columns are as they last
    // left them, not `Panes::default()` (which the saved-layout path and the tests start from).
    let record = record.unwrap_or_else(|| WindowRecord {
        id,
        layout: Layout::remembered(&app(cx).last_panes, settings(cx).sidebar_group),
        ..Default::default()
    });
    let displays: Vec<Bounds<Pixels>> = cx.displays().iter().map(|d| d.bounds()).collect();
    let saved = record.bounds.as_deref().and_then(window_bounds::decode).filter(|saved| {
        // A monitor may be gone since: never restore a window nobody can see.
        displays.iter().any(|d| d.intersects(&saved.get_bounds()))
    });
    let bounds = match (saved, source) {
        (Some(saved), _) => saved,
        (None, Some(source)) => WindowBounds::Windowed(window_bounds::cascade(source, &displays)),
        (None, None) => {
            WindowBounds::Windowed(Bounds::centered(None, size(px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1)), cx))
        }
    };
    let options = WindowOptions {
        window_bounds: Some(bounds),
        titlebar: Some(TitlebarOptions {
            title: Some("Wormhole".into()),
            appears_transparent: true,
            traffic_light_position: Some(chrome::traffic_light_position()),
        }),
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        // The title rows drag the window themselves (`chrome::TitleDrag`).
        app_owns_titlebar_drag: true,
        ..Default::default()
    };
    match crate::background::open_window(options, cx, move |window, cx| {
        cx.new(|cx| Workspace::new(record, window, cx))
    }) {
        Ok((handle, workspace)) => {
            // The system's appearance is every window's: any of them hears it change. GPUI also
            // reports it once as the window opens. Deferred, as it arrives inside this window's
            // update and may redraw the others.
            let _ = handle.update(cx, |_, window, _| {
                window.observe_window_appearance(|_, cx| cx.defer(sync_system_look)).detach();
            });
            let state = app_mut(cx);
            state.windows.push(OpenWindow { id, handle, workspace: workspace.downgrade() });
            state.front = id;
            crate::background::activate(cx);
            let _ = handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
        }
        Err(err) => tracing::error!("opening a window failed: {err:#}"),
    }
}

/// ⌘N (or the menu): a window like the one in front.
pub(super) fn new_window_from_front(cx: &mut App) {
    if AppState::with_front_workspace(cx, |ws, window, cx| ws.new_window(window, cx)).is_none()
        && app(cx).windows.is_empty()
    {
        open_workspace_window(None, None, cx);
    }
}

fn window_closed(cx: &mut App, id: WindowId) {
    let state = app_mut(cx);
    state.windows.retain(|w| w.handle.window_id() != id);
    // Sessions end with their window, so with none left there is nothing to show.
    if state.windows.is_empty() && !state.quitting {
        cx.quit();
    }
}

/// Everything that is worth keeping, for every window (scrollback included).
fn save_all(cx: &mut App) {
    app_mut(cx).quitting = true;
    let windows = app(cx).windows.clone();
    for window in &windows {
        AppState::with_workspace(window, cx, |ws, win, cx| ws.persist(win, cx));
    }
}

/// What the socket delivered: a hook's message, or a control request to answer (`control`).
fn dispatch_hook(incoming: ipc::Incoming, cx: &mut App) {
    let message = match incoming {
        ipc::Incoming::Message(message) => message,
        ipc::Incoming::Control { request, reply } => {
            // The connection waits on its thread for this; a receiver that gave up is no error.
            let _ = reply.send(control::handle(request, cx));
            return;
        }
    };
    match message {
        // A request never comes this way: the server answers it (above) instead of forwarding it.
        ipc::Message::Control(_) => {}
        ipc::Message::Activate => {
            crate::background::activate(cx);
            match AppState::front_window(cx) {
                Some(window) => {
                    let _ = window
                        .handle
                        .update(cx, |_, window, cx| crate::background::activate_window(window, cx));
                }
                None => open_workspace_window(None, None, cx),
            }
        }
        ipc::Message::Usage { usage } => {
            app_mut(cx).record_usage(usage);
            // Every window's Usage tab shows it.
            cx.refresh_windows();
        }
        // The terminal's own: its model and context window, for its title row.
        ipc::Message::Gauge { tab, gauge } => {
            let Some(owner) = AppState::find_window(cx, |ws, _| ws.owns_terminal(&tab)) else { return };
            AppState::with_workspace(&owner, cx, |ws, _, cx| ws.handle_gauge(&tab, gauge, cx));
        }
        ipc::Message::Hook { tab, agent, event, message } => {
            let Some(owner) = AppState::find_window(cx, |ws, _| ws.owns_terminal(&tab)) else { return };
            AppState::with_workspace(&owner, cx, |ws, window, cx| {
                ws.handle_hook(&tab, agent, &event, message, window, cx)
            });
        }
    }
}

/// A notification about the terminal `tab` (its id) was clicked (this runs from the platform's
/// callback, outside any window's update): bring its window and the terminal forward.
pub(super) fn open_tab(tab: i64, cx: &mut App) {
    let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) else { return };
    AppState::with_workspace(&owner, cx, |ws, window, cx| {
        crate::background::activate_window(window, cx);
        ws.reveal_terminal(tab, window, cx);
    });
}

fn handle_menu_command(command: MenuCommand, cx: &mut App) {
    tracing::debug!(?command, "menu command");
    menu_bar::run_command(command, cx);
}

/// Once per tick, whatever the number of windows.
fn tick(cx: &mut App) {
    app_mut(cx).ticks += 1;
    let ticks = app(cx).ticks;
    if SettingsStore::poll(cx) {
        cx.refresh_windows();
    }
    apply_theme(cx);
    notifications::tick(ticks, cx);
    menu_bar::sync(cx);
    app_menu::sync(cx);
    updates::schedule_check(ticks, cx);
    update_restart::tick(cx);
    if ticks.is_multiple_of(5) {
        project::prune_projects(cx);
        if crate::fonts::poll_custom(cx) {
            cx.refresh_windows();
        }
    }
}

/// Push the resolved theme to GPUI Kit (its theme is global, the settings and
/// Studio windows use it too), and the theme's appearance to every window's material.
fn apply_theme(cx: &mut App) {
    let store = cx.global::<SettingsStore>();
    let revision = store.revision;
    if app(cx).applied_theme == Some(revision) {
        return;
    }
    let (tokens, fonts) = (store.tokens.clone(), store.fonts.clone());
    crate::theme_bridge::apply(&tokens, &fonts, cx);
    vibrancy::set_material_appearance(tokens.appearance);
    app_mut(cx).applied_theme = Some(revision);
    cx.refresh_windows();
}

/// Hand the settings store what the system says now about how windows look (or what devctl
/// says in its place): the appearance Match system follows, Reduce Transparency and Increase
/// Contrast; and set GPUI's Reduce Motion flag, which its animations hold still under.
pub(super) fn sync_system_look(cx: &mut App) {
    let known = cx.global::<SettingsStore>().system();
    let stand_in = app(cx).look_override;
    let look = SystemLook {
        appearance: stand_in.appearance.or_else(vibrancy::system_appearance).unwrap_or(known.appearance),
        reduce_transparency: stand_in.reduce_transparency.unwrap_or_else(vibrancy::reduce_transparency),
        increase_contrast: stand_in.increase_contrast.unwrap_or_else(vibrancy::increase_contrast),
    };
    // GPUI Kit reads Reduce Motion once, at start, and stops following the system once the app
    // has set the flag itself (devctl does); reading it here keeps both cases in step.
    if let Some(still) = stand_in.reduce_motion.or_else(vibrancy::reduce_motion) {
        cx.set_reduce_motion(still);
    }
    if SettingsStore::set_system(cx, look) {
        apply_theme(cx);
    }
}

#[cfg(feature = "devctl")]
impl AppState {
    /// devctl's `appearance`, `reduce-transparency`, `increase-contrast` and `reduce-motion`: say
    /// this in place of the system (or follow it again), and apply it now.
    pub(crate) fn override_look(cx: &mut App, change: impl FnOnce(&mut LookOverride)) -> SystemLook {
        change(&mut app_mut(cx).look_override);
        sync_system_look(cx);
        cx.global::<SettingsStore>().system()
    }
}

/// Agent hooks an earlier version installed under the app's old codename are replaced with
/// today's, off the main thread: they keep working meanwhile (terminals still export the old
/// variables), and an agent without any is left alone.
fn upgrade_legacy_hooks(cx: &mut App) {
    cx.background_spawn(async move {
        let home = wormhole_core::agents::integrations::home();
        for (agent, result) in wormhole_core::agents::integrations::upgrade_legacy(&home) {
            match result {
                Ok(()) => tracing::info!(?agent, "replaced the hooks an earlier version installed"),
                Err(err) => {
                    tracing::warn!(?agent, "couldn't replace the hooks an earlier version installed: {err}")
                }
            }
        }
    })
    .detach();
}
