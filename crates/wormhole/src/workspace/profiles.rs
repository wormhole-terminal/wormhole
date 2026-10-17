//! Profiles in the app (RF-21): which one Wormhole is in, what each one keeps while it is not the
//! shown one, and where a window's data goes.
//!
//! One profile is shown at a time and its state is the app's own (`AppState.store`, `hub`, the
//! `SettingsStore` global, …), so everything written for a single profile keeps working. Every
//! other profile that has been opened is parked in a [`ProfileSlot`]; switching swaps the two
//! (`switch_profile`), so no call site has to know which profile it is in. A window knows its
//! profile (`Workspace.profile`) and saves into that profile's database, shown or parked.

use std::collections::HashMap;

use wormhole_core::git::worktree;
use wormhole_core::git::worktree::Worktrees;
use wormhole_core::layout::REMEMBERED_PANES_KEY;
use wormhole_core::layout::RememberedPanes;
use wormhole_core::onboarding::DISMISSED_TIPS_KEY;
use wormhole_core::onboarding::DismissedTips;
use wormhole_core::profile::{DataRoot, NewProfile, ProfileId, ProfileList, ProfilePaths};
use wormhole_core::store::Store;

use super::app::{AppState, OpenWindow};
use super::*;

impl AppState {
    /// The profile Wormhole is in.
    pub(crate) fn active_profile(&self) -> &ProfileId {
        self.profiles.active_id()
    }

    /// The database of `profile`: the live one when it is the shown profile, else the parked one.
    pub(crate) fn store_of(&self, profile: &ProfileId) -> Option<&Store> {
        if profile == self.active_profile() {
            self.store.as_ref()
        } else {
            self.parked.get(profile).and_then(|slot| slot.data.store.as_ref())
        }
    }

    fn store_of_mut(&mut self, profile: &ProfileId) -> Option<&mut Store> {
        if profile == self.profiles.active_id() {
            self.store.as_mut()
        } else {
            self.parked.get_mut(profile).and_then(|slot| slot.data.store.as_mut())
        }
    }

    /// The project folders (preferences, trust, notes sync) of `profile`.
    pub(super) fn hub_of_mut(&mut self, profile: &ProfileId) -> Option<&mut project::ProjectHub> {
        if profile == self.profiles.active_id() {
            Some(&mut self.hub)
        } else {
            self.parked.get_mut(profile).map(|slot| &mut slot.data.hub)
        }
    }

    pub(crate) fn save_window(&mut self, profile: &ProfileId, record: &WindowRecord) {
        if let Some(store) = self.store_of_mut(profile)
            && let Err(err) = store.save_window(record)
        {
            tracing::error!("saving window {} failed: {err:#}", record.id);
        }
    }

    /// A window was closed on purpose: its sessions go with it, from its own profile's database.
    pub(crate) fn forget_window(&mut self, profile: &ProfileId, id: i64) {
        if let Some(store) = self.store_of_mut(profile)
            && let Err(err) = store.delete_window(id)
        {
            tracing::error!("forgetting window {id} failed: {err:#}");
        }
    }

    /// Ids for sessions (terminals) and splits. One counter for every profile, so a terminal's id
    /// (what hooks report, what `find_session` searches for across windows) never repeats.
    pub(crate) fn alloc_id_in(&mut self, profile: &ProfileId) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        // Belt and braces with `ids_start_at`: a fresh id never comes with a notes base.
        if let Some(hub) = self.hub_of_mut(profile) {
            hub.id_allocated(id);
        }
        id
    }

    /// Make a profile (its folder and settings from `new.from`) and add it to the list. Wormhole
    /// stays in the profile it is in: the caller switches when that is wanted.
    pub(crate) fn create_profile(new: NewProfile, cx: &mut App) -> Result<ProfileId, String> {
        let state = app_mut(cx);
        let root = state.root.clone();
        state.profiles.create(&root, new).map_err(|err| err.to_string())
    }

    /// Delete a profile, its files and its windows. When Wormhole is in it, it goes to Default
    /// first. Deferred callers only (windows are updated).
    pub(crate) fn delete_profile(id: &ProfileId, cx: &mut App) -> Result<(), String> {
        if id.is_default() {
            return Err("The Default profile can't be deleted".into());
        }
        if app(cx).profiles.get(id).is_none() {
            return Err(format!("no profile {id}"));
        }
        if app(cx).active_profile() == id {
            Self::switch_profile(&ProfileId::default_profile(), cx)?;
        }
        for window in app(cx).windows_of(id).cloned().collect::<Vec<_>>() {
            let _ = window.handle.update(cx, |_, win, _| win.remove_window());
        }
        let state = app_mut(cx);
        // Its database closes before its folder goes.
        state.parked.remove(id);
        let root = state.root.clone();
        state.profiles.delete(&root, id).map(|_| ()).map_err(|err| err.to_string())?;
        cx.refresh_windows();
        Ok(())
    }

    /// Open `profile`'s database and settings when this launch hasn't yet, so a window can live in
    /// it without Wormhole being in it. The shown profile is always open.
    pub(crate) fn ensure_open(&mut self, profile: &ProfileId) {
        if profile == self.active_profile() || self.parked.contains_key(profile) {
            return;
        }
        let mut slot = ProfileSlot::open(&self.root.profile(profile));
        slot.data.hub.ids_start_at(self.next_id);
        self.parked.insert(profile.clone(), slot);
    }

    /// A new window in `target` ("Open a window in…"), stepped away from the window in front. When
    /// Wormhole is not in `target`, it is the odd window. Deferred callers only.
    pub(crate) fn open_window_in_profile(target: &ProfileId, cx: &mut App) {
        if app(cx).profiles.get(target).is_none() {
            return;
        }
        app_mut(cx).ensure_open(target);
        let target = target.clone();
        // A start folder of its own (the profile's), not the window in front's.
        if AppState::with_front_workspace(cx, |ws, window, cx| {
            let cwd = (ws.profile == target).then(|| ws.default_cwd(cx)).flatten();
            ws.new_window_in_profile(cwd, target.clone(), window, cx);
        })
        .is_none()
        {
            super::app::open_workspace_window_in(&target, None, None, cx);
        }
    }

    /// The window in front goes to `target` ("Move this window to…"): where it stays visible, as
    /// the odd window when Wormhole is not in `target`. Deferred callers only.
    pub(crate) fn move_front_window_to_profile(target: &ProfileId, cx: &mut App) -> Result<(), String> {
        if app(cx).profiles.get(target).is_none() {
            return Err(format!("no profile {target}"));
        }
        let Some(front) = Self::front_window(cx) else { return Ok(()) };
        if &front.profile == target {
            return Ok(());
        }
        app_mut(cx).ensure_open(target);
        Self::with_workspace(&front, cx, |ws, _, cx| ws.rehome(target, cx)).unwrap_or(Ok(()))?;
        if let Some(window) = app_mut(cx).windows.iter_mut().find(|w| w.id == front.id) {
            window.profile = target.clone();
            window.hidden = false;
        }
        // The window left the shown profile and may have been its last: it must not be left without
        // one (nothing to type in, and closing the odd window would then quit).
        let shown = app(cx).active_profile().clone();
        if app(cx).windows_of(&shown).next().is_none() {
            super::app::open_workspace_window_in(&shown, None, None, cx);
        }
        super::app_menu::sync(cx);
        cx.refresh_windows();
        Ok(())
    }

    /// The settings of `profile`: the live ones when it is shown, else its parked ones. A hidden
    /// profile's agents tell you by their own profile's notification settings.
    pub(crate) fn settings_of(&self, profile: &ProfileId, cx: &App) -> wormhole_core::config::Settings {
        match self.parked.get(profile) {
            Some(slot) if profile != self.active_profile() => slot.settings.settings.clone(),
            _ => settings(cx).clone(),
        }
    }

    /// The windows of `profile`, shown or not.
    pub(crate) fn windows_of<'a>(&'a self, profile: &'a ProfileId) -> impl Iterator<Item = &'a OpenWindow> {
        self.windows.iter().filter(move |w| &w.profile == profile)
    }
}

impl Workspace {
    /// This window and its terminals go to profile `target`: its record and its terminals'
    /// scrollback are saved into that profile's database and removed from the old one. The
    /// terminals themselves are untouched, so they keep the environment they started with.
    fn rehome(&mut self, target: &ProfileId, cx: &mut Context<Self>) -> Result<(), String> {
        let from = self.profile.clone();
        if &from == target {
            return Ok(());
        }
        let record = self.window_record(cx);
        let scrollback: Vec<(i64, String)> = self
            .sessions
            .iter()
            .map(|session| &session.pane)
            .map(|pane| (pane.id, pane.terminal.read(cx).scrollback_text(super::SCROLLBACK_LINES)))
            .collect();
        let state = app_mut(cx);
        let Some(store) = state.store_of_mut(target) else {
            return Err(format!("the database of profile {target} is not open"));
        };
        store.save_window(&record).map_err(|err| format!("saving the window in {target}: {err:#}"))?;
        for (id, text) in &scrollback {
            if let Err(err) = store.save_scrollback(*id, text) {
                tracing::warn!("scrollback of {id} did not move to {target}: {err:#}");
            }
        }
        state.forget_window(&from, self.window_id);
        self.profile = target.clone();
        self.dirty = false;
        cx.notify();
        Ok(())
    }
}

/// What the app keeps per profile besides the settings: its database, closed tips, project
/// folders, recent projects, plan usage, worktrees and remembered side columns.
pub(super) struct ProfileData {
    pub store: Option<Store>,
    pub tips: DismissedTips,
    pub hub: project::ProjectHub,
    pub recent: Vec<PathBuf>,
    pub usage: Vec<wormhole_core::usage::Usage>,
    pub worktrees: Worktrees,
    pub last_panes: RememberedPanes,
}

impl ProfileData {
    /// Open the profile's database and read what it keeps.
    pub fn open(paths: &ProfilePaths) -> Self {
        let store =
            Store::open_for(paths).inspect_err(|err| tracing::error!("persistence disabled: {err:#}")).ok();
        let meta = |key: &str| store.as_ref().and_then(|s| s.meta(key).ok().flatten());
        let tips = DismissedTips::parse(meta(DISMISSED_TIPS_KEY).as_deref());
        let usage = meta(wormhole_core::usage::STORE_KEY)
            .map(|json| wormhole_core::usage::from_json(&json))
            .unwrap_or_default();
        // Worktrees removed by hand since are forgotten, so nothing offers to remove them.
        let mut worktrees =
            meta(worktree::STORE_KEY).map(|json| Worktrees::from_json(&json)).unwrap_or_default();
        worktrees.prune();
        let last_panes = RememberedPanes::parse(meta(REMEMBERED_PANES_KEY).as_deref());
        let recent = menu_bar::MenuState::load_recent(store.as_ref());
        Self {
            store,
            tips,
            hub: project::ProjectHub::load_from(paths.dir()),
            recent,
            usage,
            worktrees,
            last_panes,
        }
    }
}

/// What a profile keeps while another is shown: [`ProfileData`] and its settings (the
/// `SettingsStore` global is the shown profile's).
pub(super) struct ProfileSlot {
    pub data: ProfileData,
    pub settings: SettingsStore,
}

impl ProfileSlot {
    fn open(paths: &ProfilePaths) -> Self {
        Self { data: ProfileData::open(paths), settings: SettingsStore::load_for(paths) }
    }
}

pub(super) type Parked = HashMap<ProfileId, ProfileSlot>;

impl AppState {
    /// Wormhole is in profile `target` from now on: every window of the profile that was shown goes
    /// out of sight with its terminals running, and the target's come back as they were (or are
    /// restored from its database the first time, or opened fresh when it had none). The two
    /// profiles trade places in the app's state, so everything that reads "the" settings, store or
    /// project folders reads the target's from here on. Deferred callers only: windows are updated.
    pub(crate) fn switch_profile(target: &ProfileId, cx: &mut App) -> Result<(), String> {
        let from = app(cx).active_profile().clone();
        if &from == target {
            return Ok(());
        }
        if app(cx).profiles.get(target).is_none() {
            return Err(format!("no profile {target}"));
        }
        // What a relaunch restores: the profile being left as it is now.
        for window in app(cx).windows_of(&from).cloned().collect::<Vec<_>>() {
            Self::with_workspace(&window, cx, |ws, win, cx| ws.persist(win, cx));
        }

        let paths = app(cx).root.profile(target);
        // Before the target's settings resolve their fonts and icon themes.
        crate::active_profile::set(paths.clone());
        crate::fonts::poll_custom(cx);
        let slot = match app_mut(cx).parked.remove(target) {
            Some(slot) => slot,
            None => ProfileSlot::open(&paths),
        };
        let ProfileSlot { data, settings } = slot;
        let leaving_settings = SettingsStore::swap_in(cx, settings);
        let first_free = app(cx).next_id;
        let state = app_mut(cx);
        let ProfileData { store, tips, mut hub, mut recent, usage, worktrees, last_panes } = data;
        hub.ids_start_at(first_free);
        let leaving = ProfileData {
            store: std::mem::replace(&mut state.store, store),
            tips: std::mem::replace(&mut state.tips, tips),
            hub: std::mem::replace(&mut state.hub, hub),
            usage: std::mem::replace(&mut state.usage, usage),
            worktrees: std::mem::replace(&mut state.worktrees, worktrees),
            last_panes: std::mem::replace(&mut state.last_panes, last_panes),
            recent: Vec::new(),
        };
        state.menu.switch_profile(&mut recent);
        let leaving = ProfileData { recent, ..leaving };
        state.parked.insert(from.clone(), ProfileSlot { data: leaving, settings: leaving_settings });
        if let Err(err) = state.profiles.set_active(target) {
            return Err(err.to_string());
        }
        if let Err(err) = state.profiles.save(&state.root) {
            tracing::warn!("remembering the active profile: {err}");
        }
        state.applied_theme = None;

        Self::show_profile_windows(&from, target, cx);
        super::app::sync_system_look(cx);
        super::app::apply_theme(cx);
        super::app_menu::sync(cx);
        cx.refresh_windows();
        Ok(())
    }

    /// Hide `from`'s windows and bring `target`'s forward, opening them first when it has none.
    fn show_profile_windows(from: &ProfileId, target: &ProfileId, cx: &mut App) {
        for window in app(cx).windows_of(from).cloned().collect::<Vec<_>>() {
            let _ = window.handle.update(cx, |_, win, _| crate::window_ops::hide(win));
        }
        for window in &mut app_mut(cx).windows {
            if &window.profile == from {
                window.hidden = true;
            } else if &window.profile == target {
                window.hidden = false;
            }
        }
        let shown = app(cx).windows_of(target).cloned().collect::<Vec<_>>();
        if shown.is_empty() {
            // The first time in this profile since launch: its windows come back from its database.
            let saved = app(cx)
                .store
                .as_ref()
                .and_then(|s| {
                    s.load().inspect_err(|err| tracing::error!("loading saved sessions failed: {err:#}")).ok()
                })
                .unwrap_or_default();
            if saved.is_empty() {
                super::app::open_workspace_window_in(target, None, None, cx);
            }
            for record in saved {
                super::app::open_workspace_window_in(target, Some(record), None, cx);
            }
            return;
        }
        for window in &shown {
            let _ = window.handle.update(cx, |_, win, _| crate::window_ops::show(win));
        }
        let front = shown.iter().find(|w| w.id == app(cx).front).or(shown.first()).cloned();
        if let Some(front) = front {
            crate::background::activate(cx);
            let _ = front.handle.update(cx, |_, win, cx| crate::background::activate_window(win, cx));
        }
    }
}

/// The first ids free in every profile's database: one counter serves them all, so a terminal
/// or window id never repeats across profiles. `live` is the shown profile's open database.
pub(super) fn first_free_ids(root: &DataRoot, list: &ProfileList, live: Option<&Store>) -> (i64, i64) {
    let mut ids = live.and_then(|s| s.next_ids().ok()).unwrap_or((1, 1));
    for profile in list.profiles().iter().filter(|p| &p.id != list.active_id()) {
        let paths = root.profile(&profile.id);
        if !Store::exists_for(&paths) {
            continue;
        }
        // Opened for its numbers only and closed again: it is opened for good when switched to.
        match Store::open_for(&paths).and_then(|s| s.next_ids()) {
            Ok((terminal, window)) => ids = (ids.0.max(terminal), ids.1.max(window)),
            Err(err) => tracing::warn!("reading the ids of profile {}: {err:#}", profile.id),
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use wormhole_core::profile::{NewProfile, StartFrom};
    use wormhole_core::store::{SessionRecord, WindowRecord};

    use super::{DataRoot, ProfileList, Store, first_free_ids};

    fn window(id: i64, session: i64) -> WindowRecord {
        WindowRecord { id, sessions: vec![SessionRecord::new(session, None, None)], ..Default::default() }
    }

    fn scratch(name: &str) -> DataRoot {
        let dir = std::env::temp_dir().join(format!("wormhole-profiles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        DataRoot::new(dir)
    }

    #[test]
    fn ids_never_collide_across_profiles_and_a_window_stays_in_its_profile() {
        let root = scratch("ids");
        let mut list = ProfileList::default();
        let work = list
            .create(
                &root,
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
            )
            .expect("creates the profile");
        let mut default_store = Store::open_for(&root.default_profile()).expect("opens Default's database");
        let mut work_store = Store::open_for(&root.profile(&work)).expect("opens Work's database");
        default_store.save_window(&window(1, 5)).expect("saves in Default");
        work_store.save_window(&window(7, 40)).expect("saves in Work");
        drop(work_store);

        // Default is shown: the counters start past Work's ids as well.
        let (terminal, window_id) = first_free_ids(&root, &list, Some(&default_store));
        assert!(terminal > 40 && window_id > 7, "got {terminal}, {window_id}");

        // A window saved in Work is not loaded when Default opens.
        let loaded = default_store.load().expect("loads Default");
        assert_eq!(loaded.iter().map(|w| w.id).collect::<Vec<_>>(), vec![1]);
        let _ = std::fs::remove_dir_all(root.base());
    }
}
