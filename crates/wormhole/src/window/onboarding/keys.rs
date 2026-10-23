//! The intro window's keys. It holds the keyboard while it is open, but only takes what it uses:
//! ← and → between the pages, ⏎ for the main button, R to replay, Esc to skip, ⌘O on the folder
//! step. ⇥ and ⇧⇥ walk its buttons and stay among them (the sidebar and terminal under it are out of
//! reach), and ⏎ or Space press the one the keys are on, as everywhere. The window's own shortcuts
//! wait until the intro is gone, so nothing reaches the terminal underneath.
//!
//! ⇥ is a binding in the page's own key context, which wins over the Kit's at the window's root: GPUI
//! runs bindings before a key listener sees the key, so a listener could only see focus already gone.

use gpui_kit::*;
use wormhole_core::onboarding::intro::Start;
use wormhole_core::onboarding::intro::page::Entrance;
use wormhole_core::onboarding::{Event, Step};

use super::{FADE_OUT, FirstRun};
use crate::actions;

/// The most buttons the page is walked through when ⇥ looks for its stops.
const STOPS_MAX: usize = 32;

actions!(intro_window, [NextButton, PreviousButton]);

/// The page's key context: where ⇥ and ⇧⇥ stay among its buttons.
pub(super) const CONTEXT: &str = "IntroWindow";

/// Bind ⇥ and ⇧⇥ in the page's context.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextButton, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousButton, Some(CONTEXT)),
    ]);
}

impl FirstRun {
    /// Whether the page is still coming in: a key or a click then cuts the animation short instead
    /// of acting.
    pub(in crate::window) fn intro_is_entering(&self, cx: &App) -> bool {
        self.is_open() && self.intro_entrance(cx).least() < 1.0 && crate::intro::progress(cx).is_some()
    }

    /// How far each part of the page has entered now.
    pub(in crate::window) fn intro_entrance(&self, cx: &App) -> Entrance {
        let Some(progress) = crate::intro::progress(cx) else { return Entrance::DONE };
        if progress.skipped {
            return Entrance::DONE;
        }
        // A replay's window is still showing, and leaving, with the page it had.
        if progress.start == Start::Shown && progress.seconds < FADE_OUT.as_secs_f32() {
            return Entrance::DONE;
        }
        wormhole_core::onboarding::intro::page::entrance(progress.style, progress.seconds)
    }

    /// Keys while the intro window is open.
    pub(in crate::window) fn intro_keys(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(intro) = &self.intro else { return };
        let keys = &event.keystroke;
        let m = &keys.modifiers;
        let key = keys.key.as_str();
        // ⌘Q is the menu's.
        if m.platform && key == "q" {
            return;
        }
        // ⇥ and ⇧⇥ are the page's bindings (`init`).
        if key == "tab" {
            return;
        }
        // While the page comes in, a key only cuts the animation short.
        if self.intro_is_entering(cx) {
            crate::intro::skip(cx);
            cx.stop_propagation();
            return;
        }
        // The page's own keys work where the keyboard is on the page, not on a button or in a menu
        // that has keys of its own.
        let on_page = intro.focus.is_focused(window);
        let step = intro.model.step();
        let plain = !m.modified();
        let handled = match key {
            "enter" if plain && on_page => {
                let event = if intro.model.is_last() { Event::Start } else { Event::Continue };
                self.onboarding_event(event, window, cx);
                true
            }
            // The end of the intro is its own button's, not an arrow's.
            "right" if plain && on_page => {
                if !intro.model.is_last() {
                    self.onboarding_event(Event::Continue, window, cx);
                }
                true
            }
            "left" if plain && on_page => {
                self.onboarding_event(Event::Back, window, cx);
                true
            }
            "r" if plain && on_page => {
                self.replay_intro(window, cx);
                true
            }
            "escape" if plain && on_page => {
                self.onboarding_event(Event::Skip, window, cx);
                true
            }
            "o" if m.platform && step == Step::Folder => {
                self.pick_folder(cx);
                true
            }
            // The shortcuts of the window wait.
            _ => m.modified(),
        };
        if handled {
            cx.stop_propagation();
        }
    }

    /// ⇥ (or ⇧⇥ when `back`): the next button of the page, wrapping from its last to its first.
    /// The window's own ⇥ would go on into the sidebar under the page.
    pub(super) fn tab_within_intro(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(intro) = &self.intro else { return };
        let page = intro.focus.clone();
        let current = window.focused(cx);
        // The page's stops in order: what ⇥ visits from the page itself, until it would leave it.
        let mut stops: Vec<FocusHandle> = Vec::new();
        window.focus(&page, cx);
        for _ in 0..STOPS_MAX {
            window.focus_next(cx);
            let Some(next) = window.focused(cx) else { break };
            if page.is_focused(window) || !page.contains_focused(window, cx) || stops.contains(&next) {
                break;
            }
            stops.push(next);
        }
        let at = current.and_then(|current| stops.iter().position(|stop| *stop == current));
        let n = stops.len();
        let target = match (at, back) {
            (None, false) => 0,
            (None, true) => n.saturating_sub(1),
            (Some(i), false) => (i + 1) % n.max(1),
            (Some(i), true) => (i + n - 1) % n.max(1),
        };
        match stops.get(target) {
            Some(stop) => window.focus(stop, cx),
            None => window.focus(&page, cx),
        }
    }

    /// Esc that reached the page from a button or a menu that left it alone: the keyboard goes back
    /// to the page, and a second Esc skips (the Esc ladder, one layer at a time).
    pub(super) fn intro_escape(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(intro) = &self.intro else { return };
        if event.keystroke.key != "escape" || event.keystroke.modifiers.modified() {
            return;
        }
        if !intro.focus.is_focused(window) {
            window.focus(&intro.focus, cx);
            cx.stop_propagation();
        }
    }
}
