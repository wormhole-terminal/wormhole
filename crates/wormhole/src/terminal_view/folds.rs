//! Folded output in the view: asking for a fold, opening and closing one. What a fold is, and the rules
//! about what may be folded, are `wormhole_term::folds`; how its marker is drawn is `render::folds`.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_term::folds::FoldOwner;
use wormhole_text::{keys, whole};

use super::TerminalView;
use crate::settings::settings_in;

impl TerminalView {
    /// Whether the Fold output feature is on in this build.
    pub(super) fn folds_on(&self, cx: &App) -> bool {
        settings_in(&self.profile, cx).feature_enabled(FeatureFlag::OutputFolds)
    }

    /// Show what the settings say: without the feature, or with "Never hide lines", the view shows every
    /// line and nothing is folded. Called when the terminal starts and when the settings change.
    pub(crate) fn sync_folds(&self, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let off = !self.folds_on(cx) || settings_in(&self.profile, cx).never_hide_lines;
        if pty.set_folds_off(off) {
            cx.notify();
        }
    }

    /// A Probe's hook folds the output of finished block `block`. The Probe holds the `fold` permission
    /// (the dispatcher checked), and this terminal's rules say what may be folded; a refusal is not shown.
    pub(crate) fn fold_for_probe(
        &mut self,
        block: u64,
        owner: FoldOwner,
        keep: (u32, u32),
        cx: &mut Context<Self>,
    ) {
        let Some(pty) = &self.pty else { return };
        if !self.folds_on(cx) {
            return;
        }
        match pty.fold_block_output_by(block, owner, keep) {
            Ok(_) => cx.notify(),
            Err(refused) => tracing::debug!(?refused, "a Probe's fold was refused"),
        }
    }

    /// Edit → Show Hidden Lines: open every fold of this terminal. Says how many lines that showed.
    pub(super) fn show_hidden_lines(&mut self, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let hidden: usize = self.folded_rows().into_iter().sum();
        if pty.fold_show_all() {
            crate::platform::a11y::announce(&keys::terminal_fold_shown(whole(hidden)), false);
            cx.notify();
        }
    }

    /// The terminal's `Pty`, for a test that feeds it what its shell would print.
    #[cfg(test)]
    pub(crate) fn pty_for_test(&self) -> Option<&wormhole_term::Pty> {
        self.pty.as_ref()
    }

    /// How many rows each closed fold in the last frame hides.
    fn folded_rows(&self) -> Vec<usize> {
        let state = self.render_state.borrow();
        let folds = state.snapshot().rows().iter().filter_map(|row| row.fold());
        folds.filter(|fold| !fold.expanded).map(|fold| fold.hidden()).collect()
    }

    /// A bell, or a program's report that it needs input: what is hidden near the end of the output is
    /// shown, so the person can find what is asking.
    pub(super) fn open_recent_folds(&self, cx: &mut Context<Self>) {
        if !self.folds_on(cx) {
            return;
        }
        if let Some(pty) = self.pty.as_ref().filter(|pty| pty.has_folds()) {
            pty.fold_expand_recent();
            cx.notify();
        }
    }

    /// Output came: while folds exist, look at whether a program is reading a secret (echo off), which
    /// opens the recent folds and refuses new ones. The system tells no change of echo, so output is when
    /// it is looked at, and only for a terminal that has folds.
    pub(super) fn check_secret_input(&self, cx: &mut Context<Self>) {
        // Without the feature no fold exists, and an output event takes no lock for them.
        if !self.folds_on(cx) {
            return;
        }
        if let Some(pty) = self.pty.as_ref().filter(|pty| pty.has_folds())
            && pty.refresh_fold_secure()
        {
            cx.notify();
        }
    }

    /// The probes that may fold (by id) have changed: the others' folds in this terminal go.
    pub(crate) fn retain_probe_folds(&self, probes: &[String], cx: &mut Context<Self>) {
        if let Some(pty) = self.pty.as_ref().filter(|pty| pty.has_folds()) {
            pty.fold_retain_probes(probes);
            cx.notify();
        }
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
