//! The window's side of the feature contract (`feature`): making the features' entities with the
//! window, hearing their events, and asking the table what the frame needs (the tab list, the action
//! bindings, the menu, the keys over a list). Nothing here names a feature.

use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::menus::MenuState;

use crate::settings::store_in;
use crate::window::Workspace;
use crate::window::feature::{FeatureDef, FeatureEvent, FeatureState, TabDef};

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

    /// Subscribes the window to a feature's entity, for its [`FeatureEvent`]s. A feature's `make`
    /// calls this on the entity it creates, and keeps the subscription in its [`FeatureState`].
    pub(in crate::window) fn listen<T: EventEmitter<FeatureEvent>>(
        entity: &Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(entity, window, |this, _, event: &FeatureEvent, window, cx| {
            this.on_feature_event(event, window, cx);
        })
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
            FeatureEvent::SendText { session, text } => {
                if let Some(ix) = self.find_session(*session) {
                    self.sessions[ix].terminal().update(cx, |terminal, _| terminal.paste(text));
                }
            }
        }
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

    /// Lets every feature with a `menu` hook say what it hides or disables, whether it is on or off:
    /// the facts carry the mask.
    pub(in crate::window) fn feature_menus(&self, state: &mut MenuState, cx: &App) {
        let facts = self.facts(cx);
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
        let features = store_in(&self.profile, cx).features();
        for (at, def) in self.features.iter().enumerate() {
            if !features.has(def.id) {
                continue;
            }
            if let (Some(focus_in), Some(entity)) =
                (def.state.as_ref().and_then(|state| state.focus_in), self.feature_entity(at))
                && focus_in(entity, session, window, cx)
            {
                return true;
            }
        }
        false
    }
}
