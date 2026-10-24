//! What VoiceOver hears beyond the accessibility tree GPUI builds from the views' roles and
//! labels: announcements, since GPUI has no live regions. A terminal that needs you is
//! said at once, interrupting; one that finished waits its turn. Which changes are said, and in
//! what words, is core's (`wormhole_core::notify::Announcer`); this only posts them. And
//! whether macOS's Keyboard navigation is on, which makes buttons ⇥ stops.
//!
//! The system's Reduce Motion, Increase Contrast and Reduce Transparency are read and watched in
//! `look` with the rest of what the system says about how windows look.

use super::background_mode;
use super::os::a11y as os;

/// Say `text` through VoiceOver (when it runs; otherwise nothing happens). `interrupt` cuts into
/// what it is saying (needs you); without it the announcement is queued (finished).
pub fn announce(text: &str, interrupt: bool) {
    tracing::debug!(interrupt, "announce: {text}");
    #[cfg(feature = "devctl")]
    devctl::remember(text, interrupt);
    // A test instance in background mode keeps them to devctl (`a11y said`); tests never speak.
    if !cfg!(test) && !background_mode::on() {
        os::post(text, interrupt);
    }
}

/// Whether GPUI builds the accessibility tree for the frame being drawn (an app has asked the window for
/// its accessibility tree). GPUI rebuilds the tree from scratch in every frame's prepaint and does not
/// replay the nodes of a view it reuses, so a view kept between frames would drop out of the tree: callers
/// that keep one draw it afresh while this is true. GPUI forces a frame when it turns on.
///
/// On macOS it never turns off again: GPUI's platform drops the deactivation, so for the window's life it
/// stays true once any client has connected, and that is any app using the accessibility API (a screen
/// reader, a switch or voice control, an automation or inspection tool), not only VoiceOver.
pub fn tree_built(window: &gpui_kit::Window) -> bool {
    #[cfg(test)]
    if forced::get() {
        return true;
    }
    window.is_a11y_active()
}

/// In a test, say the tree is built (GPUI's test platform has no assistive app to connect), until the
/// returned guard is dropped: a panic cannot leave the next test on this thread with it set.
#[cfg(test)]
pub fn force_tree() -> ForcedTree {
    forced::set(true);
    ForcedTree(())
}

/// The tree is said to be built while this lives (`force_tree`).
#[cfg(test)]
#[must_use = "the tree is built only while the guard lives"]
pub struct ForcedTree(());

#[cfg(test)]
impl Drop for ForcedTree {
    fn drop(&mut self) {
        forced::set(false);
    }
}

#[cfg(test)]
mod forced {
    use std::cell::Cell;

    thread_local! {
        static ON: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn get() -> bool {
        ON.get()
    }

    pub(super) fn set(on: bool) {
        ON.set(on);
    }
}

/// Whether macOS's Keyboard navigation is on (System Settings → Keyboard): ⇥ then reaches buttons
/// too, the title row's among them. AppKit can't be watched for it, so it is read when ⇥
/// is pressed; it is cheap.
pub fn keyboard_navigation() -> bool {
    #[cfg(feature = "devctl")]
    if let Some(on) = devctl::KEYBOARD_NAVIGATION.get() {
        return on;
    }
    // Tests never ask the real system: buttons are not ⇥ stops there.
    !cfg!(test) && os::keyboard_navigation()
}

/// devctl's `keyboard-navigation`: say it is on or off, or (`None`) follow the system again.
#[cfg(feature = "devctl")]
pub fn set_keyboard_navigation(on: Option<bool>) {
    devctl::KEYBOARD_NAVIGATION.set(on);
}

/// The announcements made lately, oldest first, for devctl's `a11y said`.
#[cfg(feature = "devctl")]
pub fn recent() -> Vec<(String, bool)> {
    devctl::recent()
}

/// Wake the window's accessibility tree, as VoiceOver or Accessibility Inspector would by asking
/// for it: GPUI builds the tree only once something has. It is there from the next frame on.
#[cfg(feature = "devctl")]
pub fn wake_tree(window: &gpui_kit::Window) {
    os::wake_tree(window);
}

#[cfg(feature = "devctl")]
mod devctl {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    /// Enough to see what a scripted check caused.
    const KEEP: usize = 16;

    thread_local! {
        static SAID: RefCell<VecDeque<(String, bool)>> = const { RefCell::new(VecDeque::new()) };
        /// What devctl says in place of the system's Keyboard navigation.
        pub(super) static KEYBOARD_NAVIGATION: Cell<Option<bool>> = const { Cell::new(None) };
    }

    pub(super) fn remember(text: &str, interrupt: bool) {
        SAID.with_borrow_mut(|said| {
            if said.len() == KEEP {
                said.pop_front();
            }
            said.push_back((text.to_string(), interrupt));
        });
    }

    pub(super) fn recent() -> Vec<(String, bool)> {
        SAID.with_borrow(|said| said.iter().cloned().collect())
    }
}
