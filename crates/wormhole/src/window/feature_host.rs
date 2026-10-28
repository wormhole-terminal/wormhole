//! The window's side of the feature contract (`feature`): making the features' entities with the
//! window, hearing their events, and asking the table what the frame needs (the tab list, the action
//! bindings, the menu, the keys over a list). Nothing here names a feature.

use std::any::TypeId;
use std::path::Path;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::SessionRecord;

use crate::actions::gpui_action;
use crate::app_state::{AppState, app};
use crate::sidebar::{RowExtras, RowInfo};
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{
    CloseAlso, CloseExtra, CloseOf, CoverDef, FeatureDef, FeatureEvent, RepoRequest, Root, SpawnAdd, Stake,
    TabDef,
};

/// Every window looks again at what its features show of the world outside the app, for a change that
/// concerns them all (a project's trust or files). Deferred: it may be called from inside a window's own
/// update, and each window's lease is taken in turn.
pub(crate) fn refresh_all_features(cx: &mut App) {
    cx.defer(|cx| {
        for window in app(cx).windows.clone() {
            AppState::with_workspace(&window, cx, |ws, _, cx| ws.refresh_features(cx));
        }
    });
}

/// Every window of `profile` is drawn again, for a change in what they all show that none of them watches
/// (a global such as the profile's project hub). Deferred for the same reason.
pub(crate) fn redraw_windows_of(profile: &ProfileId, cx: &mut App) {
    let profile = profile.clone();
    cx.defer(move |cx| {
        let windows: Vec<_> =
            app(cx).windows.iter().filter(|window| window.profile == profile).cloned().collect();
        for window in windows {
            AppState::with_workspace(&window, cx, |ws, _, cx| ws.mark_dirty(cx));
        }
    });
}

/// The listener of a tab's action (`TabDef::show`): shows the tab whose action ran. Typed and without
/// captures, so binding it allocates nothing; the window and the tab are found when the action runs.
pub(crate) fn show_tab<A: Action>(action: &A, window: &mut Window, cx: &mut App) {
    let handle = window.window_handle();
    let found = app(cx).windows.iter().find(|open| open.handle == handle).map(|open| open.workspace.clone());
    if let Some(workspace) = found {
        workspace.update(cx, |ws, cx| ws.show_tab_of_action(action, cx)).ok();
    }
}

/// A feature's entity and the window's hold on its events. Dropping it unsubscribes. The only way to
/// make one is [`Workspace::listen`] (the subscription field is private to this module), so the
/// subscription is the window's and no feature can forget it or keep the entity without it.
pub(crate) struct FeatureState {
    pub entity: AnyEntity,
    _events: Subscription,
    /// A change the feature notes redraws the window: it is the window that draws its panel.
    _redraw: Subscription,
}

impl Workspace {
    /// What the table says this window does for `id`; `None` for a feature that hasn't moved out of
    /// `Workspace`'s fields yet.
    #[cfg_attr(not(test), expect(dead_code, reason = "only the table's tests ask by id"))]
    pub(in crate::window) fn feature_def(&self, id: FeatureId) -> Option<&'static FeatureDef> {
        self.features.iter().find(|def| def.id == id)
    }

    /// Makes the entity of every feature that has state, in the table's order, for the window being
    /// made. The window calls each `make` once, whether the feature is on or off.
    pub(in crate::window) fn make_feature_states(
        features: &'static [FeatureDef],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Option<FeatureState>> {
        features.iter().map(|def| def.state.as_ref().map(|state| (state.make)(window, cx))).collect()
    }

    /// Subscribes the window to a feature's entity, for its [`FeatureEvent`]s, and gives back the
    /// entity with that subscription. A feature's `make` ends with it.
    pub(crate) fn listen<T: EventEmitter<FeatureEvent>>(
        entity: &Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> FeatureState {
        let events = cx.subscribe_in(entity, window, |this, _, event: &FeatureEvent, window, cx| {
            this.on_feature_event(event, window, cx);
        });
        let redraw = cx.observe(entity, |_, _, cx| cx.notify());
        FeatureState { entity: entity.clone().into(), _events: events, _redraw: redraw }
    }

    /// What the window does when a feature says something. Runs after the feature's own update has
    /// finished (GPUI delivers events when the outermost update ends), so the `Workspace` is free.
    fn on_feature_event(&mut self, event: &FeatureEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            FeatureEvent::ShowTab(tab) => self.show_tab(*tab, cx),
            FeatureEvent::FocusTerminal => self.focus_terminal(window, cx),
            FeatureEvent::Dirty => self.mark_dirty(cx),
            FeatureEvent::Editing(editing) => {
                self.feature_editing = *editing;
                cx.notify();
            }
            FeatureEvent::SendText { session, text, submit } => {
                self.send_to_terminal(*session, text, *submit, window, cx);
            }
            FeatureEvent::TabGone(tab) => {
                if self.right_tab == *tab {
                    self.set_right_tab(TabId::FALLBACK);
                }
                cx.notify();
            }
            FeatureEvent::Terminal { target, input } => {
                self.run_in_terminal(target.clone(), input.clone(), window, cx);
            }
            FeatureEvent::OpenTerminals { terminals } => {
                self.open_terminals(terminals.clone(), window, cx);
            }
            FeatureEvent::Repo(request) => self.on_repo_request(request, cx),
        }
    }

    /// A feature asked for the window's reading of a terminal's repository to be done or changed.
    fn on_repo_request(&mut self, request: &RepoRequest, cx: &mut Context<Self>) {
        match request {
            RepoRequest::Refresh { session } => {
                if let Some(ix) = self.find_session(*session) {
                    self.refresh_git(ix, cx);
                }
            }
            RepoRequest::Compare { session, branch } => {
                if let Some(ix) = self.find_session(*session) {
                    self.compare_with(ix, branch.clone(), cx);
                }
            }
        }
    }

    /// The feature that has something laid over the terminal now, with its entity.
    fn open_cover(&self, cx: &App) -> Option<(&'static CoverDef, &AnyEntity)> {
        let features = crate::settings::store_in(&self.profile, cx).features();
        self.features.iter().enumerate().find_map(|(at, def)| {
            let (cover, entity) = (def.cover.as_ref()?, self.feature_entity(at)?);
            (features.has(def.id) && (cover.open)(entity, cx)).then_some((cover, entity))
        })
    }

    /// Whether a feature has something laid over the terminal (a diff): the terminal and its panes are
    /// not drawn, find and the tab bar's focus stop are not offered, and Esc closes it first.
    pub(in crate::window) fn covered(&self, cx: &App) -> bool {
        self.open_cover(cx).is_some()
    }

    /// The file the cover shows (a diff's), where it has one: what the title row's entry for it names.
    #[cfg(any(test, debug_assertions))]
    pub(in crate::window) fn cover_file(&self, cx: &App) -> Option<std::path::PathBuf> {
        let (cover, entity) = self.open_cover(cx)?;
        (cover.watch)(entity, cx)
    }

    /// What is laid over the terminal, built for this frame.
    pub(in crate::window) fn render_cover(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let (cover, entity) = self.open_cover(cx)?;
        let entity = entity.clone();
        Some((cover.view)(&entity, &self.frame_facts(window, cx), window, cx))
    }

    /// The title row's entry while something is laid over the terminal.
    pub(in crate::window) fn render_cover_title(&self, cx: &mut App) -> Option<AnyElement> {
        let (cover, entity) = self.open_cover(cx)?;
        let entity = entity.clone();
        (cover.title)(&entity, &self.facts(cx), cx)
    }

    /// Takes down what is laid over the terminal and gives the terminal the keyboard. Whether there
    /// was something.
    pub(in crate::window) fn close_cover(&self, window: &mut Window, cx: &mut App) -> bool {
        let Some((cover, entity)) = self.open_cover(cx) else { return false };
        let entity = entity.clone();
        (cover.close)(&entity, &self.facts(cx), window, cx);
        true
    }

    /// The panel's tab was shown: each feature drops what only belongs to its own tab being on show.
    pub(in crate::window) fn tabs_changed(&self, tab: TabId, cx: &mut App) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(changed), Some(entity)) = (def.tab_changed, self.feature_entity(at)) {
                changed(entity, tab, cx);
            }
        }
    }

    /// The files the features want looked at in the background hop of the status read for the terminal
    /// on screen.
    pub(in crate::window) fn repo_watched(&self, cx: &App) -> Option<std::path::PathBuf> {
        self.features.iter().enumerate().find_map(|(at, def)| {
            let (cover, entity) = (def.cover.as_ref()?, self.feature_entity(at)?);
            (cover.watch)(entity, cx)
        })
    }

    /// The window read the repository status of the terminal on screen: each feature that follows it
    /// looks at the answer.
    pub(in crate::window) fn repo_was_read(
        &self,
        session: i64,
        observed: Option<crate::window::repo::Observed>,
        cx: &mut App,
    ) {
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(cover), Some(entity)) = (def.cover.as_ref(), self.feature_entity(at)) {
                (cover.read)(entity, &facts, session, observed.as_ref(), cx);
            }
        }
    }

    /// Pastes `text` into terminal `session` (nothing for blank text), presses Enter after it with
    /// `submit`, and gives the terminal the keyboard.
    pub(in crate::window) fn send_to_terminal(
        &mut self,
        session: i64,
        text: &str,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim();
        let Some(ix) = self.find_session(session).filter(|_| !text.is_empty()) else { return };
        let terminal = self.sessions[ix].terminal().clone();
        super::feature_terminals::paste_text(&terminal, text, submit, cx);
        window.focus(&terminal.focus_handle(cx), cx);
    }

    /// The table's entry for `tab` and its place in the table: the entry whose feature brings it.
    pub(crate) fn tab_entry(&self, tab: TabId) -> Option<(usize, &'static TabDef)> {
        self.features
            .iter()
            .enumerate()
            .find_map(|(at, def)| def.tabs.iter().find(|candidate| candidate.id == tab).map(|t| (at, t)))
    }

    /// The entity of the feature at `at` in the table, if it has state.
    pub(crate) fn feature_entity(&self, at: usize) -> Option<&AnyEntity> {
        self.feature_states.get(at)?.as_ref().map(|state| &state.entity)
    }

    /// Whether the panel offers `tab` now: its feature is in the table and on, and the tab says it is
    /// `shown`. A window made without the feature has no such tab.
    pub(in crate::window) fn tab_offered(&self, tab: TabId, facts: &WindowFacts<'_>, cx: &App) -> bool {
        self.tab_entry(tab)
            .is_some_and(|(at, entry)| facts.features.has(self.features[at].id) && (entry.shown)(facts, cx))
    }

    /// Shows the tab whose action is `action`: what the key or the menu line of a tab does.
    fn show_tab_of_action(&mut self, action: &dyn Action, cx: &mut Context<Self>) {
        let tab = (self.features.iter().flat_map(|def| def.tabs))
            .find(|tab| gpui_action(tab.action).is_some_and(|shows| shows.partial_eq(action)))
            .map(|tab| tab.id);
        if let Some(tab) = tab {
            self.show_tab(tab, cx);
        }
    }

    /// The window's actions for the table's tabs and its features' own, bound on `root`: every
    /// offered tab's action shows the tab, then each feature's `bind` adds the rest.
    pub(in crate::window) fn bind_features(&self, mut root: Root, cx: &mut Context<Self>) -> Root {
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if !facts.features.has(def.id) {
                continue;
            }
            for tab in def.tabs {
                if (tab.shown)(&facts, cx) {
                    root = (tab.show)(root);
                }
            }
            if let (Some(bind), Some(entity)) = (def.bind, self.feature_entity(at)) {
                root = bind(entity, &facts, root, cx);
            }
        }
        root
    }

    /// What the menus, the palette and the sheet leave out for features that are off (the keymap's
    /// rows tagged with them), then what each feature with a `menu` hook hides or disables, on or
    /// off: the facts carry the mask.
    pub(in crate::window) fn feature_menus(&self, state: &mut MenuState, cx: &App) {
        let facts = self.facts(cx);
        state.hidden.extend(facts.features.hidden_actions());
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(menu), Some(entity)) = (def.menu, self.feature_entity(at)) {
                menu(entity, &facts, state, cx);
            }
        }
    }

    /// A terminal other than the one on screen was selected: each feature clears what it kept for
    /// the one before.
    pub(in crate::window) fn features_selected(&self, session: i64, cx: &mut App) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(selected), Some(entity)) =
                (def.state.as_ref().and_then(|state| state.selected), self.feature_entity(at))
            {
                selected(entity, session, cx);
            }
        }
    }

    /// ⇧⌘E from the terminal: the first feature that is on and takes the keys for the terminal on
    /// screen gets them. Whether one did.
    pub(in crate::window) fn focus_feature(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let session = self.active_session().id;
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if !facts.features.has(def.id) {
                continue;
            }
            if let (Some(focus_in), Some(entity)) =
                (def.state.as_ref().and_then(|state| state.focus_in), self.feature_entity(at))
                && focus_in(entity, &facts, session, window, cx)
            {
                return true;
            }
        }
        false
    }

    /// Once per window tick: the housekeeping of every feature with state, on or off.
    pub(crate) fn tick_features(&self, window: &mut Window, cx: &mut App) {
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(tick), Some(entity)) = (def.tick, self.feature_entity(at)) {
                tick(entity, &facts, window, cx);
            }
        }
    }

    /// The window looks again at what its features show of the world outside the app: when it comes to
    /// the front, when an outside change should reach every window (a project's trust or files).
    pub(crate) fn refresh_features(&self, cx: &mut App) {
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(refresh), Some(entity)) = (def.refresh, self.feature_entity(at)) {
                refresh(entity, &facts, cx);
            }
        }
    }

    /// The window was moved to another profile: the features that keep something of the window for its
    /// profile follow it. The facts are the new profile's.
    pub(super) fn rehomed_features(&self, cx: &mut App) {
        let facts = self.facts(cx);
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(rehomed), Some(entity)) =
                (def.profiles.as_ref().and_then(|profiles| profiles.rehomed), self.feature_entity(at))
            {
                rehomed(entity, &facts, cx);
            }
        }
    }

    /// What the features add to the start of a terminal in `cwd`: their environment, in the table's
    /// order, and the shell of the last that names one.
    pub(crate) fn spawn_adds(
        profile: &ProfileId,
        cwd: Option<&Path>,
        features: &[FeatureDef],
        cx: &mut App,
    ) -> SpawnAdd {
        let mut all = SpawnAdd::default();
        for add in features.iter().filter_map(|def| def.spawn).map(|spawn| spawn(profile, cwd, cx)) {
            all.env.extend(add.env);
            all.shell = add.shell.or(all.shell);
        }
        all
    }

    /// Whether the folder of terminal `session` is one a feature has approved to run programs for (git's
    /// pull request lookup asks before it starts `gh`). It fails closed: with no feature that gives the
    /// approval, or one that doesn't, nothing is approved.
    pub(crate) fn programs_approved(&self, session: i64, cx: &App) -> bool {
        let mut approved = false;
        for (at, def) in self.features.iter().enumerate() {
            let Some(approves) = def.approves_programs else { continue };
            if !self.feature_entity(at).is_some_and(|entity| approves(entity, session, &self.profile, cx)) {
                return false;
            }
            approved = true;
        }
        approved
    }

    /// The saved record of the terminal at `ix`: its own, then each feature's part of it, written by
    /// every feature that has state (on or off: what an off feature kept is kept).
    pub(in crate::window) fn session_record(&self, ix: usize, cx: &App) -> SessionRecord {
        let session = &self.sessions[ix];
        let mut record = session.record(cx);
        // The app installs the table at launch. Without one, no feature writes its part, and a save
        // would replace the notes and tasks stored for the terminal with none. Never silently.
        #[cfg(not(test))]
        if self.features.is_empty() {
            tracing::error!(
                "saving terminal {} with no feature table: its notes and tasks are left out",
                session.id
            );
        }
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(state), Some(entity)) = (def.state.as_ref(), self.feature_entity(at)) {
                (state.record)(entity, session.id, &mut record, cx);
            }
        }
        record
    }

    /// A terminal was opened here from `record`: each feature with state reads its part. After `make`.
    pub(in crate::window) fn features_opened(
        &self,
        record: &SessionRecord,
        window: &mut Window,
        cx: &mut App,
    ) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(state), Some(entity)) = (def.state.as_ref(), self.feature_entity(at)) {
                (state.opened)(entity, record, window, cx);
            }
        }
    }

    /// The terminal `session` is closed for good: each feature drops its state and what it stores.
    pub(in crate::window) fn features_closed(&self, session: i64, cx: &mut App) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(state), Some(entity)) = (def.state.as_ref(), self.feature_entity(at)) {
                (state.closed)(entity, session, cx);
            }
        }
    }

    /// The terminal `session` lives on in another window: each feature drops its state here and
    /// deletes nothing; the other window reads it back from the record.
    pub(in crate::window) fn features_left(&self, session: i64, cx: &mut App) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(state), Some(entity)) = (def.state.as_ref(), self.feature_entity(at)) {
                (state.left)(entity, session, cx);
            }
        }
    }

    /// What closing these terminals would lose, by each feature that is on and has something at stake.
    pub(crate) fn close_stakes(&self, sessions: &[i64], cx: &App) -> Vec<Stake> {
        let features = crate::settings::store_in(&self.profile, cx).features();
        let mut stakes = Vec::new();
        for (at, def) in self.features.iter().enumerate() {
            if !features.has(def.id) {
                continue;
            }
            if let (Some(close_stake), Some(entity)) =
                (def.close.as_ref().and_then(|close| close.stake), self.feature_entity(at))
            {
                stakes.extend(close_stake(entity, sessions, cx));
            }
        }
        stakes
    }

    /// What each feature that is on adds to the question before closing `sessions` (the terminal, or a
    /// group's): in the table's order.
    pub(crate) fn close_extras(&self, sessions: &[i64], of: CloseOf, cx: &App) -> Vec<CloseExtra> {
        let facts = self.facts(cx);
        let mut extras = Vec::new();
        for (at, def) in self.features.iter().enumerate() {
            if !facts.features.has(def.id) {
                continue;
            }
            if let (Some(close_extra), Some(entity)) =
                (def.close.as_ref().and_then(|close| close.extra), self.feature_entity(at))
            {
                extras.extend(close_extra(entity, &facts, sessions, of, cx));
            }
        }
        extras
    }

    /// Lets each feature that is on put its extra on the rows (a count after the status).
    pub(crate) fn feature_row_extras(&self, rows: &mut [RowInfo], cx: &App) {
        let facts = self.facts(cx);
        let mut extras = RowExtras::new(rows);
        for (at, def) in self.features.iter().enumerate() {
            if !facts.features.has(def.id) {
                continue;
            }
            if let (Some(row_extras), Some(entity)) = (def.row_extras, self.feature_entity(at)) {
                row_extras(entity, &facts, &mut extras, cx);
            }
        }
    }

    /// The window's entity of feature type `T`, if the table has one. For code that reaches a feature
    /// from outside the frame (devctl, a bridge between two features); the frame itself goes by the
    /// table.
    pub(crate) fn feature<T: 'static>(&self) -> Option<Entity<T>> {
        // The type is compared before the handle is cloned: a feature's view asks every frame.
        let state = self
            .feature_states
            .iter()
            .flatten()
            .find(|state| state.entity.entity_type() == TypeId::of::<T>())?;
        state.entity.clone().downcast::<T>().ok()
    }
}

/// What the features' extras make of the close question: the sentence they add after the rest of its
/// detail, and the button of the first that has one (with what to do once the terminals are closed).
pub(in crate::window) fn extras_to_ask(extras: Vec<CloseExtra>) -> (String, Option<CloseAlso>) {
    let note = extras.iter().map(|extra| extra.note.as_str()).collect::<Vec<_>>().join(" ");
    let also = extras.into_iter().find_map(|extra| extra.also);
    (note, also)
}
