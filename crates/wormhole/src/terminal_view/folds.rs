//! Folded output in the view: asking for a fold, opening and closing one. What a fold is, and the rules
//! about what may be folded, are `wormhole_term::folds`; how its marker is drawn is `render::folds`.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_text::{keys, whole};

use super::TerminalView;
use crate::settings::settings_in;

impl TerminalView {
    /// Whether the Fold output feature is on in this build.
    pub(super) fn folds_on(&self, cx: &App) -> bool {
        settings_in(&self.profile, cx).feature_enabled(FeatureFlag::OutputFolds)
    }

    /// Fold the output of finished block `id`. A refusal is not an error to show: the menu offers the
    /// item only for a block the rules can take, and one that has since scrolled onto the screen says
    /// no here, which is the rule.
    pub(super) fn fold_output(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        match pty.fold_block_output(id) {
            Ok(_) => cx.notify(),
            Err(refused) => tracing::info!(?refused, "fold output refused"),
        }
    }

    /// The marker was clicked (or VoiceOver's Show): open or close fold `id`. Opening says so.
    pub(super) fn toggle_fold(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let was_open = self.fold_state(id);
        if pty.fold_toggle(id) {
            if let Some((hidden, false)) = was_open {
                crate::platform::a11y::announce(&keys::terminal_fold_shown(whole(hidden)), false);
            }
            cx.notify();
        }
    }

    /// How many rows fold `id` hides and whether it is open, as the last frame showed it.
    fn fold_state(&self, id: u64) -> Option<(usize, bool)> {
        let state = self.render_state.borrow();
        let fold = state.snapshot().rows().iter().find_map(|row| row.fold().filter(|fold| fold.id == id))?;
        Some((fold.hidden(), fold.expanded))
    }
}
