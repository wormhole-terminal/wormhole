//! The ⌘1–9 hints: keycaps that show only while ⌘ is held on its own for a moment.

use std::time::{Duration, Instant};

/// How long ⌘ has to be held alone before the keycaps show, so a quick ⌘C or ⌘T never
/// flashes them.
pub const HOLD_DELAY: Duration = Duration::from_millis(250);

/// Whether the hints show: ⌘ pressed on its own, held for [`HOLD_DELAY`], with no other key
/// pressed meanwhile. They go on release, on any other key, and when the window loses focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CmdHold {
    /// When ⌘ alone went down.
    since: Option<Instant>,
    /// Another key was pressed while ⌘ was held: no hints until ⌘ is pressed again.
    spoiled: bool,
}

impl CmdHold {
    /// The modifier keys changed; `cmd_alone` is whether ⌘ is now the only one held.
    pub fn modifiers(&mut self, cmd_alone: bool, now: Instant) {
        if !cmd_alone {
            *self = Self::default();
        } else if self.since.is_none() {
            *self = Self { since: Some(now), spoiled: false };
        }
    }

    /// A key went down (other than a modifier).
    pub fn key(&mut self) {
        if self.since.is_some() {
            self.spoiled = true;
        }
    }

    /// The window lost focus: whatever is held, it is not for this window any more.
    pub fn blur(&mut self) {
        *self = Self::default();
    }

    pub fn showing(&self, now: Instant) -> bool {
        !self.spoiled && self.since.is_some_and(|since| now.saturating_duration_since(since) >= HOLD_DELAY)
    }

    /// While ⌘ is held and the hints are still to come, how long until they show.
    pub fn due_in(&self, now: Instant) -> Option<Duration> {
        let since = self.since.filter(|_| !self.spoiled)?;
        let held = now.saturating_duration_since(since);
        (held < HOLD_DELAY).then(|| HOLD_DELAY - held)
    }
}

/// A terminal's shortcut as assistive technology names it (`aria-keyshortcuts`).
pub fn keyshortcut(number: usize) -> String {
    format!("Meta+{number}")
}

/// The keycap a numbered terminal shows.
pub fn keycap(number: usize) -> String {
    format!("\u{2318}{number}")
}

#[cfg(test)]
mod tests {
    use super::{CmdHold, HOLD_DELAY, keycap, keyshortcut};
    use std::time::{Duration, Instant};

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn hints_show_after_holding_cmd_alone() {
        let t0 = Instant::now();
        let mut hold = CmdHold::default();
        assert!(!hold.showing(t0));
        hold.modifiers(true, t0);
        assert!(!hold.showing(t0 + 249 * MS));
        assert_eq!(hold.due_in(t0 + 200 * MS), Some(50 * MS));
        assert!(hold.showing(t0 + HOLD_DELAY));
        assert_eq!(hold.due_in(t0 + HOLD_DELAY), None);
        // Still the same press: the clock doesn't restart.
        hold.modifiers(true, t0 + 400 * MS);
        assert!(hold.showing(t0 + 400 * MS));
        // Released.
        hold.modifiers(false, t0 + 500 * MS);
        assert!(!hold.showing(t0 + 600 * MS));
    }

    #[test]
    fn a_quick_shortcut_never_flashes_them() {
        let t0 = Instant::now();
        let mut hold = CmdHold::default();
        hold.modifiers(true, t0);
        hold.key(); // ⌘C
        assert!(!hold.showing(t0 + 300 * MS));
        assert_eq!(hold.due_in(t0 + 100 * MS), None);
        // Any other key while they show hides them too.
        let mut hold = CmdHold::default();
        hold.modifiers(true, t0);
        assert!(hold.showing(t0 + 300 * MS));
        hold.key();
        assert!(!hold.showing(t0 + 301 * MS));
        // Pressing ⌘ again starts over.
        hold.modifiers(false, t0 + 400 * MS);
        hold.modifiers(true, t0 + 500 * MS);
        assert!(hold.showing(t0 + 750 * MS));
    }

    #[test]
    fn another_modifier_or_losing_focus_hides_them() {
        let t0 = Instant::now();
        let mut hold = CmdHold::default();
        hold.modifiers(true, t0);
        hold.modifiers(false, t0 + 300 * MS); // ⇧ joined ⌘
        assert!(!hold.showing(t0 + 300 * MS));
        hold.modifiers(true, t0 + 400 * MS); // ⇧ let go, ⌘ alone again: a new hold
        assert!(!hold.showing(t0 + 500 * MS));
        assert!(hold.showing(t0 + 650 * MS));
        hold.blur();
        assert!(!hold.showing(t0 + 700 * MS));
    }

    #[test]
    fn shortcuts_are_spelled_for_the_keycap_and_for_voiceover() {
        assert_eq!(keycap(1), "⌘1");
        assert_eq!(keyshortcut(9), "Meta+9");
    }
}
