//! A window's tick (every 400 ms): what changes without an event to say so (folders, agents, git,
//! projects, the window's look), and the saves that follow it.
//!
//! Part of the window: it may use the shared state (`app`) and the platform, and must not use the
//! surfaces above it (the command palette, the shortcut sheet, the Settings and Studio windows, the
//! status item, the app root). A terminal the person has seen is told through `app_state::attention`.

use gpui_kit::*;

use crate::app_state::attention::{self, Attention};
use crate::settings::{settings_in, ui_in};
use crate::window::Workspace;
use crate::window::save::SCROLLBACK_SAVE_INTERVAL;

impl Workspace {
    /// Apply settings that live outside GPUI's render tree.
    fn sync_appearance(&mut self, cx: &mut Context<Self>) {
        let blur = crate::style::wants_blur(settings_in(&self.profile, cx), true);
        self.blur.set_enabled(blur);
        // Without the blur the window stands on the panel's colour: the gaps round the terminal's
        // surface and between a split's panes show the window, glass or not, and would otherwise
        // show the desktop through it.
        self.blur.set_backing((!blur).then(|| ui_in(&self.profile, cx).panel));
    }

    pub(super) fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ticks += 1;
        self.sync_appearance(cx);
        self.track_window(window, cx);
        // Folders are also re-read when a terminal prints (see `TerminalView`); this
        // is the backstop. Agent detection walks the process tree, so less often.
        let refresh_agent = self.ticks % 5 == 1;
        let window_active = window.is_window_active();
        if window_active && !self.is_front(cx) {
            self.became_front(cx);
        }
        let mut changed = false;
        // Only the selected terminal and the panes shown with it are on screen.
        let on_screen: Vec<bool> = (0..self.sessions.len()).map(|ix| self.on_screen(ix)).collect();
        // A program's clipboard read is asked about only where the question can be seen.
        let looking = crate::focus_keys::in_front(window);
        for (session, is_active) in self.sessions.iter().zip(on_screen) {
            let pane = &session.pane;
            changed |= pane.terminal.update(cx, |terminal, _| {
                // A split joined or left since the selection last changed.
                terminal.set_on_screen(is_active);
                terminal.set_in_front(is_active && looking);
                let before = terminal.status();
                let mut changed = terminal.tick(is_active, window_active);
                if is_active && window_active {
                    changed |= terminal.acknowledge();
                }
                if terminal.refresh_cwd() {
                    changed = true;
                    self.dirty = true;
                }
                if refresh_agent {
                    changed |= terminal.refresh_agent();
                }
                changed || before != terminal.status()
            });
            if is_active && window_active {
                attention::tell(Attention::Seen(pane.id), cx);
            }
        }
        // A tab whose words change with time (Usage's "resets 34m") is redrawn on its own period.
        if changed || self.tab_refresh_due() {
            cx.notify();
        }
        self.sync_close_dot(window, cx);
        self.leave_missing_tab(cx);
        self.poll_repos(cx);
        if self.ticks % 5 == 1 {
            // A `git init` in the folder a session is already in changes its root without a `cd`.
            for session in &self.sessions {
                session.repo_root.forget();
            }
        }
        // The features' housekeeping (Notes: the backstop for a new note that never got the focus it
        // would be thrown away on losing).
        self.tick_features(window, cx);
        self.tick_extras(cx);
        self.refresh_status_bar(cx);
        self.crumb_tick(cx);
        self.sidebar_tick(window_active, cx);
        if self.dirty && !self.discarded {
            self.save(cx);
        }
        if self.last_scrollback_save.elapsed() > SCROLLBACK_SAVE_INTERVAL {
            self.save_scrollback(cx);
        }

        #[cfg(feature = "devctl")]
        self.run_tick_hook(window, cx);
    }
}
