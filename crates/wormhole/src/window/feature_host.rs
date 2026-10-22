//! The window's side of the feature contract (`feature`): making the features' entities with the
//! window, hearing their events, and asking the table what the frame needs (the tab list, the action
//! bindings, the menu, the keys over a list). Nothing here names a feature.

use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::storage::store::SessionRecord;

use crate::actions::gpui_action;
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureDef, FeatureEvent, Root, Stake, TabDef};
use crate::window::sidebar::{RowExtras, RowInfo};

/// The pause between a pasted text and the Enter that submits it.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

/// A feature's entity and the window's hold on its events. Dropping it unsubscribes. The only way to
/// make one is [`Workspace::listen`] (the subscription field is private to this module), so the
/// subscription is the window's and no feature can forget it or keep the entity without it.
pub(in crate::window) struct FeatureState {
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
    pub(in crate::window) fn listen<T: EventEmitter<FeatureEvent>>(
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
        terminal.update(cx, |terminal, _| terminal.paste(text));
        if submit {
            // Give the program a moment to take the paste in before Enter.
            let terminal = terminal.clone();
            cx.spawn(async move |_, cx| {
                cx.background_executor().timer(SUBMIT_DELAY).await;
                terminal.update(cx, |terminal, _| terminal.write(b"\r".to_vec()));
            })
            .detach();
        }
        window.focus(&terminal.focus_handle(cx), cx);
    }

    /// The table's entry for `tab` and its place in the table: the entry whose feature brings it.
    pub(in crate::window) fn tab_entry(&self, tab: TabId) -> Option<(usize, &'static TabDef)> {
        self.features
            .iter()
            .enumerate()
            .find_map(|(at, def)| def.tabs.iter().find(|candidate| candidate.id == tab).map(|t| (at, t)))
    }

    /// The entity of the feature at `at` in the table, if it has state.
    pub(in crate::window) fn feature_entity(&self, at: usize) -> Option<&AnyEntity> {
        self.feature_states.get(at)?.as_ref().map(|state| &state.entity)
    }

    /// Whether the panel offers `tab` now, by the table's entry when it has one and by the window's
    /// own rule when the feature has not moved into the table yet.
    pub(in crate::window) fn tab_offered(&self, tab: TabId, facts: &WindowFacts<'_>, cx: &App) -> bool {
        match self.tab_entry(tab) {
            Some((at, entry)) => facts.features.has(self.features[at].id) && (entry.shown)(facts, cx),
            None => self.legacy_tab_offered(tab, cx),
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
                let Some(action) = gpui_action(tab.action).filter(|_| (tab.shown)(&facts, cx)) else {
                    continue;
                };
                let id = tab.id;
                root = root.on_boxed_action(
                    &*action,
                    cx.listener(move |this, _: &dyn Action, _, cx| this.show_tab(id, cx)),
                );
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
    pub(in crate::window) fn tick_features(&self, window: &mut Window, cx: &mut App) {
        for (at, def) in self.features.iter().enumerate() {
            if let (Some(tick), Some(entity)) = (def.tick, self.feature_entity(at)) {
                tick(entity, window, cx);
            }
        }
    }

    /// The saved record of the terminal at `ix`: its own, then each feature's part of it, written by
    /// every feature that has state (on or off: what an off feature kept is kept).
    pub(in crate::window) fn session_record(&self, ix: usize, cx: &App) -> SessionRecord {
        let session = &self.sessions[ix];
        let mut record = session.record(cx);
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
    pub(in crate::window) fn close_stakes(&self, sessions: &[i64], cx: &App) -> Vec<Stake> {
        let features = crate::settings::store_in(&self.profile, cx).features();
        let mut stakes = Vec::new();
        for (at, def) in self.features.iter().enumerate() {
            if !features.has(def.id) {
                continue;
            }
            if let (Some(close_stake), Some(entity)) = (def.close_stake, self.feature_entity(at)) {
                stakes.extend(close_stake(entity, sessions, cx));
            }
        }
        stakes
    }

    /// Lets each feature that is on put its extra on the rows (a count after the status).
    pub(in crate::window) fn feature_row_extras(&self, rows: &mut [RowInfo], cx: &App) {
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
        self.feature_states.iter().flatten().find_map(|state| state.entity.clone().downcast::<T>().ok())
    }
}
