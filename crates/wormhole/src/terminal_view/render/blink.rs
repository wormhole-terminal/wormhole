//! Blinking text (SGR 5 and 6): one steady clock, shared by every terminal, that fades the glyphs
//! of blinking cells in its off half.
//!
//! The phase is a function of the time, not state on the rows, so a phase change repaints and
//! shapes nothing: rows keep what they were built with (`Glyph::blink` marks the glyphs) and the
//! painter picks the color. Rapid blink uses the same clock; there is no second rate. A timer
//! runs only while something on screen blinks, the window is in front and Reduce Motion is off;
//! otherwise blinking text is steady.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui_kit::{Context, Hsla};

use crate::terminal_view::TerminalView;

/// Each half of a cycle: 500 ms on, 500 ms faded, a second in all.
const PHASE: Duration = Duration::from_millis(500);

/// How much of its color the text keeps in the off half; faded, not gone, so it stays readable.
const FADED_ALPHA: f32 = 0.25;

/// Whether the blink clock should run: something on screen blinks, the window is in front, and
/// the system hasn't asked for less motion.
pub(super) fn wanted(any_blink: bool, window_active: bool, reduce_motion: bool) -> bool {
    any_blink && window_active && !reduce_motion
}

/// Whether the clock is in its off half `since` the start. Every terminal counts from the same
/// start, so panes blink together.
fn faded_after(since: Duration) -> bool {
    (since.as_millis() / PHASE.as_millis()) % 2 == 1
}

fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Whether blinking text is in its off half now.
pub(super) fn faded_now() -> bool {
    faded_after(start().elapsed())
}

/// A color in the off half.
pub(super) fn faded(color: Hsla) -> Hsla {
    Hsla { a: color.a * FADED_ALPHA, ..color }
}

impl TerminalView {
    /// After each frame: run the clock while `wanted`, stop it (dropping its task cancels the
    /// timer) when not, so a screen without blinking text costs no wakeups.
    pub(super) fn text_blink_wanted(&mut self, wanted: bool, cx: &mut Context<Self>) {
        self.wants_text_blink = wanted;
        if !wanted {
            self.text_blink = None;
            return;
        }
        if self.text_blink.is_some() {
            return;
        }
        let first = start();
        self.text_blink = Some(cx.spawn(async move |this, cx| {
            loop {
                // Sleep to the next change of half, so the repaint lands as the phase turns.
                let into = first.elapsed().as_millis() % PHASE.as_millis();
                let wait = PHASE.saturating_sub(Duration::from_millis(into as u64));
                cx.background_executor().timer(wait.max(Duration::from_millis(1))).await;
                let running = this.update(cx, |this, cx| {
                    if !this.wants_text_blink {
                        this.text_blink = None;
                        return false;
                    }
                    this.notify_redraw(cx);
                    true
                });
                if !matches!(running, Ok(true)) {
                    return;
                }
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui_kit::hsla;

    use super::{faded, faded_after, wanted};

    #[test]
    fn a_cycle_is_a_second_half_on_half_faded() {
        let at = |ms| faded_after(Duration::from_millis(ms));
        assert_eq!(
            [at(0), at(499), at(500), at(999), at(1000), at(1500)],
            [false, false, true, true, false, true]
        );
    }

    #[test]
    fn the_clock_runs_only_for_blinking_text_in_a_window_in_front_with_motion_allowed() {
        assert!(wanted(true, true, false));
        assert!(!wanted(false, true, false), "nothing blinks: no timer at all");
        assert!(!wanted(true, false, false), "a window behind another stays steady");
        assert!(!wanted(true, true, true), "Reduce Motion keeps the text steady");
    }

    #[test]
    fn fading_keeps_the_hue_and_dims_the_alpha() {
        let color = hsla(0.3, 0.5, 0.5, 0.8);
        let dim = faded(color);
        assert_eq!((dim.h, dim.s, dim.l), (color.h, color.s, color.l));
        assert!(dim.a < color.a && dim.a > 0.0);
    }
}
