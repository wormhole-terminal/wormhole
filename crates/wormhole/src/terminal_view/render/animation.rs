//! Animated pictures (kitty's `a=f`, `a=a`, `a=c`): which frame of each is on show, and the timer
//! that repaints when it changes.
//!
//! The engine keeps each picture's plan (frames' gaps, state, loops: `wormhole_vt::image::Animation`)
//! and has no clock; the pixels are in the store. This is the clock. For every animated picture in
//! view it follows the plan with [`Animation::tick`]: the frame shown, since when, and what comes
//! next, and tells the engine which frame it is on, so a program that stops the animation leaves it
//! there. Time is the wall clock of the view, kept per view: two panes showing the same picture are
//! two animations.
//!
//! The clock runs only while something on screen animates **and** the window is in front **and**
//! Reduce Motion is off (as blinking text's does, see `blink`); otherwise an animated picture
//! stands on the frame the engine says is current. A tick repaints the pane's picture paints; the
//! rows of text are not rebuilt (they are cached by the screen's revision, which a tick does not
//! change).

use std::collections::HashMap;
use std::time::Instant;

use gpui_kit::Context;
use wormhole_term::Pty;
use wormhole_term::types::{Animation, AnimationState, Position, Tick};

use crate::terminal_view::TerminalView;

/// A picture being played.
struct Playing {
    /// The plan's generation this follows; a new one starts over.
    generation: u64,
    position: Position,
    /// When `position`'s frame came on.
    since: Instant,
    /// The frame last told to the engine.
    told: usize,
}

/// What to draw for one picture, and when to look again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Shown {
    /// The frame on show, counted from 0.
    pub frame: usize,
    /// When it changes, if it will.
    pub due: Option<Instant>,
    /// The frame is not the one the engine was last told.
    pub tell: bool,
}

/// The pictures being played in one pane.
#[derive(Default)]
pub(super) struct Clocks {
    playing: HashMap<u32, Playing>,
}

impl Clocks {
    /// The frame of picture `key` to show at `now`. With `motion` off, or a plan that is stopped,
    /// that is the plan's current frame; otherwise it moves with time.
    pub(super) fn frame(&mut self, key: u32, plan: &Animation, now: Instant, motion: bool) -> Shown {
        let last = plan.frame_count().saturating_sub(1);
        if !motion || plan.state() == AnimationState::Stopped {
            self.playing.remove(&key);
            return Shown { frame: plan.current().min(last), due: None, tell: false };
        }
        let playing = self.playing.entry(key).or_insert_with(|| Playing::start(plan, now));
        if playing.generation != plan.generation() {
            *playing = Playing::start(plan, now);
        }
        // Whole passes that went by while nobody looked (a hidden window) are skipped at once; the
        // frames of the last one are stepped through, each step a constant-time `tick`.
        let lag = now.saturating_duration_since(playing.since);
        if lag >= plan.pass_duration() {
            let skipped = plan.skip_passes(playing.position, lag);
            playing.position = skipped.position;
            playing.since = if skipped.finished { now } else { now - skipped.lag };
        }
        let mut due = None;
        // At most the frames of a pass and the ones that follow without a gap.
        for _ in 0..plan.frame_count() * 2 + 2 {
            match plan.tick(playing.position) {
                Tick::Hold => break,
                Tick::After { wait, next } => {
                    let at = playing.since + wait;
                    if at > now {
                        due = Some(at);
                        break;
                    }
                    playing.position = next;
                    playing.since = at;
                }
            }
        }
        if due.is_none() && !matches!(plan.tick(playing.position), Tick::Hold) {
            // Still behind (a plan that changed under us): the frame on show starts now.
            playing.since = now;
            due = Some(now);
        }
        let frame = playing.position.frame.min(last);
        let tell = frame != playing.told;
        playing.told = frame;
        Shown { frame, due, tell }
    }

    /// Forget the pictures that are no longer in view.
    pub(super) fn retain(&mut self, mut keep: impl FnMut(u32) -> bool) {
        self.playing.retain(|key, _| keep(*key));
    }
}

impl Playing {
    fn start(plan: &Animation, now: Instant) -> Playing {
        let position = plan.start();
        Playing { generation: plan.generation(), position, since: now, told: position.frame }
    }
}

/// The frames to draw for the pictures in view (store keys), and when the first of them changes.
pub(super) struct Frames {
    pub shown: HashMap<u32, usize>,
    pub due: Option<Instant>,
}

/// Decide the frame of each animated picture among `keys` (the pictures this frame draws).
/// `animated` says whether the store holds any animation at all: none, none of this costs a
/// lock.
pub(super) fn choose(
    keys: impl Iterator<Item = u32>,
    animated: bool,
    pty: &Pty,
    clocks: &mut Clocks,
    now: Instant,
    motion: bool,
) -> Frames {
    let mut frames = Frames { shown: HashMap::new(), due: None };
    if !animated {
        clocks.retain(|_| false);
        return frames;
    }
    let mut seen = Vec::new();
    for key in keys {
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let Some(plan) = pty.animation(key) else { continue };
        let shown = clocks.frame(key, &plan, now, motion);
        if shown.tell {
            pty.set_animation_frame(key, shown.frame);
        }
        frames.shown.insert(key, shown.frame);
        frames.due = match (frames.due, shown.due) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
    clocks.retain(|key| seen.contains(&key));
    frames
}

impl TerminalView {
    /// After each frame: the time the next animation frame is due, or `None`. Runs the timer
    /// while there is one and the window is in front, stops it (dropping the task cancels the
    /// sleep) when not, so a screen without a moving picture costs no wakeups.
    pub(super) fn image_animation_due(
        &mut self,
        due: Option<Instant>,
        window_active: bool,
        cx: &mut Context<Self>,
    ) {
        self.image_due = due.filter(|_| window_active);
        if self.image_due.is_none() {
            self.image_clock = None;
            return;
        }
        if self.image_clock.is_some() {
            return;
        }
        self.image_clock = Some(cx.spawn(async move |this, cx| {
            loop {
                let due = this.update(cx, |this, _| this.image_due).ok().flatten();
                let Some(due) = due else {
                    this.update(cx, |this, _| this.image_clock = None).ok();
                    return;
                };
                let wait = due.saturating_duration_since(Instant::now());
                cx.background_executor().timer(wait.max(std::time::Duration::from_millis(1))).await;
                let running = this.update(cx, |this, cx| {
                    // Until the next frame says when, there is nothing to wait for: a pane that
                    // is not drawn any more (hidden) lets the timer end.
                    this.image_due = None;
                    this.notify_redraw(cx);
                });
                if running.is_err() {
                    return;
                }
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Clocks, Shown};
    use wormhole_term::types::{AnimationState, Tick};

    /// A plan made the way a program makes one: a picture, frames with gaps, then `a=a`.
    fn plan(gaps: &[u32], control: &str) -> wormhole_term::types::Animation {
        let mut bytes = b"\x1b_Ga=t,f=24,s=1,v=1,i=1,q=2;AAAA\x1b\\".to_vec();
        for gap in gaps {
            bytes.extend(format!("\x1b_Ga=f,f=24,s=1,v=1,i=1,z={gap},q=2;AAAA\x1b\\").as_bytes());
        }
        bytes.extend(format!("\x1b_G{control}\x1b\\").as_bytes());
        wormhole_term::testing::animation_plan(&bytes).expect("the picture has a plan")
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_stopped_animation_stands_on_its_current_frame() {
        let a = plan(&[10, 10], "a=a,i=1,c=2,q=2");
        let mut clocks = Clocks::default();
        let now = Instant::now();
        assert_eq!(clocks.frame(1, &a, now, true), Shown { frame: 1, due: None, tell: false });
        assert_eq!(a.state(), AnimationState::Stopped);
        assert_eq!(a.tick(a.start()), Tick::Hold);
    }

    #[test]
    fn reduce_motion_keeps_a_running_animation_on_the_current_frame() {
        let a = plan(&[10, 10], "a=a,i=1,s=3,c=2,q=2");
        let mut clocks = Clocks::default();
        let now = Instant::now();
        assert_eq!(clocks.frame(1, &a, now, false).frame, 1);
        assert_eq!(clocks.frame(1, &a, now + ms(500), false), Shown { frame: 1, due: None, tell: false });
    }

    #[test]
    fn a_running_animation_moves_with_time_and_says_when_it_moves_next() {
        // The root frame has no gap until it is set: 20 ms here; the others 10 and 30.
        let a = plan(&[10, 30], "a=a,i=1,s=3,r=1,z=20,q=2");
        let mut clocks = Clocks::default();
        let t0 = Instant::now();
        assert_eq!(clocks.frame(1, &a, t0, true), Shown { frame: 0, due: Some(t0 + ms(20)), tell: false });
        let second = clocks.frame(1, &a, t0 + ms(25), true);
        assert_eq!((second.frame, second.due, second.tell), (1, Some(t0 + ms(30)), true));
        assert!(!clocks.frame(1, &a, t0 + ms(26), true).tell, "told once");
        let third = clocks.frame(1, &a, t0 + ms(31), true);
        assert_eq!((third.frame, third.due), (2, Some(t0 + ms(60))));
        // Round again: the cadence keeps its place (frame 0 came on at 60).
        let wrapped = clocks.frame(1, &a, t0 + ms(61), true);
        assert_eq!((wrapped.frame, wrapped.due), (0, Some(t0 + ms(80))));
    }

    #[test]
    fn a_frame_that_came_late_is_skipped_to_the_one_that_is_due() {
        let a = plan(&[10, 10, 10], "a=a,i=1,s=3,r=1,z=10,q=2");
        let mut clocks = Clocks::default();
        let t0 = Instant::now();
        clocks.frame(1, &a, t0, true);
        let late = clocks.frame(1, &a, t0 + ms(25), true);
        assert_eq!(late.frame, 2, "frames 0 and 1 have gone by");
        assert_eq!(late.due, Some(t0 + ms(30)));
    }

    #[test]
    fn a_pane_that_was_away_for_a_long_time_starts_again_instead_of_catching_up() {
        let a = plan(&[1, 1], "a=a,i=1,s=3,r=1,z=1,q=2");
        let mut clocks = Clocks::default();
        let t0 = Instant::now();
        clocks.frame(1, &a, t0, true);
        let back = clocks.frame(1, &a, t0 + Duration::from_secs(3600), true);
        assert!(back.due.is_some(), "it is still moving");
        assert!(back.frame < a.frame_count());
    }

    #[test]
    fn the_passes_end_on_the_last_frame_and_a_new_generation_starts_over() {
        let a = plan(&[10], "a=a,i=1,s=3,v=2,r=1,z=10,q=2");
        let mut clocks = Clocks::default();
        let t0 = Instant::now();
        clocks.frame(1, &a, t0, true);
        let done = clocks.frame(1, &a, t0 + ms(25), true);
        assert_eq!((done.frame, done.due), (1, None), "v=2 is one pass: it stays on the last frame");
        let restarted = plan(&[10], "a=a,i=1,s=3,v=2,r=1,z=10,q=2");
        assert_ne!(restarted.generation(), 0);
        let mut other = Clocks::default();
        assert_eq!(other.frame(1, &restarted, t0, true).frame, 0);
    }

    #[test]
    fn a_picture_that_leaves_the_view_is_forgotten() {
        let a = plan(&[10], "a=a,i=1,s=3,r=1,z=10,q=2");
        let mut clocks = Clocks::default();
        clocks.frame(1, &a, Instant::now(), true);
        clocks.retain(|_| false);
        assert!(clocks.playing.is_empty());
    }
}
