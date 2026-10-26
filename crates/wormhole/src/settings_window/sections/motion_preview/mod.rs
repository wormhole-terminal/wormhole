//! Settings → Motion's previews: under each choice, a small stage made of the app's own rows, heads
//! and tiles that loops through the thing the choice is about. Each scene is a file (`insert`, `fold`,
//! `status`, `column`) that says where its parts are at a moment of the loop; this is what they share:
//! the view that runs the loop, the frame and the spots the parts are put on.
//!
//! A preview is a view of its own, so the clock that redraws it is its own and the Settings window's
//! page is not what ticks. It reads the time itself instead of animating built elements: a scene is a
//! function of the loop's time `t` (0.0..1.0) that builds only what shows at `t`, so a part that is
//! away (a row not there yet, the statuses it is not in) is not in the layout, the accessibility tree
//! or running an animation of its own. `t = 0` is the picture before anything moves, which is what
//! Reduce Motion holds. A preview exists while its page is open (`MotionPreviews`), so nothing ticks
//! while another page is.
//!
//! Picking another option starts the loop again from `t = 0`. The terminal opening and the group folding
//! draw every option from the same poses the sidebar does (core's `view::motion::pose`), so the preview
//! shows exactly what the list will do; the status and the column still draw what the app does today
//! until the motion they name is built: a later phase adds a variant to the scene it belongs to.

mod column;
mod fold;
mod insert;
mod status;

use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::view::motion::{MotionChoice, Span};
use wormhole_core::view::sidebar::Density;
use wormhole_text::keys::{
    SETTINGS_MOTION_COLUMN_PREVIEW_LABEL, SETTINGS_MOTION_FOLD_PREVIEW_LABEL,
    SETTINGS_MOTION_INSERT_PREVIEW_LABEL, SETTINGS_MOTION_PREVIEW_CAPTION,
    SETTINGS_MOTION_STATUS_PREVIEW_LABEL,
};
use wormhole_text::text;

use super::samples::{CARGO_TEST, FIX_PILE, SCRATCH, SIDEBAR_CARDS, Sample};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::ui;
use crate::style::motion::ANIMATION_FPS;
use crate::style::{self, Metrics, TextRole, TileFace};

/// The widest a preview grows: the sidebar's rows read best at about the width the sidebar has.
const STAGE_MAX_W: f32 = 480.0;

/// The length of every scene's loop: long enough to rest before and after each change.
const LOOP: Duration = Duration::from_secs(4);

/// How often a still preview (Reduce Motion) looks again whether it may move.
const STILL_CHECK: Duration = Duration::from_secs(1);

/// How far `elapsed` is through a motion of `total` length (a motion of none is over at once), as the
/// poses of core take it.
fn progress(elapsed: Duration, total: Duration) -> f32 {
    if total.is_zero() { 1.0 } else { elapsed.as_secs_f32() / total.as_secs_f32() }
}

/// How long it has been at `t` of the loop since `start`, a share of it: nothing before it.
fn since(t: f32, start: f32) -> Duration {
    Duration::from_secs_f32((t - start).max(0.0) * LOOP.as_secs_f32())
}

/// `len` after `start`, as a stretch of the loop.
fn span(start: f32, len: Duration) -> Span {
    Span::new(start, start + len.as_secs_f32() / LOOP.as_secs_f32())
}

/// The sidebar's rows at the density most people have, in cards.
fn metrics() -> Metrics {
    Metrics::of(Density::Regular, true)
}

/// The agent's terminals of the scenes: working, and asking for input, as the sample list has them.
const WORKING: Sample = Sample { agent: Some(AgentId::Claude), ..SIDEBAR_CARDS };
const ASKING: Sample = Sample { agent: Some(AgentId::Claude), ..FIX_PILE };
const RUNNING: Sample = CARGO_TEST;
const IDLE: Sample = SCRATCH;

/// The sidebar's own row for `sample`, as a scene draws it: a terminal without an agent is the terminal
/// symbol. `id` tells the row's tile from the others in the window.
#[track_caller]
fn row(sample: &Sample, id: &str, ui: UiColors, cx: &App) -> AnyElement {
    sample.row(id, ui, metrics(), &TileFace::Terminal, false, cx)
}

/// Where a part of a scene is on the stage, and how much of it shows.
#[derive(Clone, Copy)]
struct Spot {
    left: f32,
    top: f32,
    /// A part with a width of its own; `None` runs across the stage.
    width: Option<f32>,
    height: f32,
    opacity: f32,
    /// The share of its height that is open: the rest is cut away from the bottom.
    clip: f32,
}

impl Spot {
    /// Across the stage, `top` points from its top.
    fn row(top: f32, height: f32) -> Self {
        Self { left: 0.0, top, width: None, height, opacity: 1.0, clip: 1.0 }
    }

    /// `width` wide, from the stage's left edge.
    fn wide(width: f32, height: f32) -> Self {
        Self { left: 0.0, top: 0.0, width: Some(width), height, opacity: 1.0, clip: 1.0 }
    }

    /// Only the top `share` of it shows.
    fn clipped(self, share: f32) -> Self {
        Self { clip: share, ..self }
    }

    fn moved(self, dx: f32, dy: f32) -> Self {
        Self { left: self.left + dx, top: self.top + dy, ..self }
    }

    fn showing(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }

    /// What `content` builds, there: nothing at all, not even built, while none of it shows.
    #[track_caller]
    fn put(self, content: impl FnOnce() -> AnyElement) -> Option<AnyElement> {
        (self.opacity > 0.0).then(|| {
            let part = div()
                .absolute()
                .left(px(self.left))
                .top(px(self.top))
                .h(px(self.height * self.clip))
                .when(self.clip < 1.0, |part| part.overflow_hidden())
                .when_some(self.width, |part, width| part.w(px(width)))
                .when(self.width.is_none(), |part| part.right_0())
                .child(div().flex_none().h(px(self.height)).child(content()));
            if self.opacity < 1.0 { part.opacity(self.opacity) } else { part }.into_any_element()
        })
    }
}

/// The four previews of the page, as long as the page is up.
pub(in crate::settings_window) struct MotionPreviews {
    insert: Entity<MotionPreview>,
    fold: Entity<MotionPreview>,
    status: Entity<MotionPreview>,
    column: Entity<MotionPreview>,
}

impl MotionPreviews {
    pub(in crate::settings_window) fn new(cx: &mut App) -> Self {
        let mut view = |kind| cx.new(|cx| MotionPreview::new(kind, cx));
        Self {
            insert: view(Scene::Insert),
            fold: view(Scene::Fold),
            status: view(Scene::Status),
            column: view(Scene::Column),
        }
    }

    /// The preview of `scene`, to put under its row.
    pub(in crate::settings_window) fn of(&self, scene: Scene, ui: UiColors) -> AnyElement {
        let view = match scene {
            Scene::Insert => &self.insert,
            Scene::Fold => &self.fold,
            Scene::Status => &self.status,
            Scene::Column => &self.column,
        };
        div()
            .w_full()
            .max_w(px(STAGE_MAX_W))
            .flex()
            .flex_col()
            .gap_2()
            .child(view.clone())
            .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_MOTION_PREVIEW_CAPTION)))
            .into_any_element()
    }
}

/// What a preview is a preview of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::settings_window) enum Scene {
    Insert,
    Fold,
    Status,
    Column,
}

impl Scene {
    /// The option of the scene's choice in use, as its place among the options.
    fn shown(self, cx: &App) -> usize {
        fn at<T: MotionChoice>(current: T) -> usize {
            T::ALL.iter().position(|choice| *choice == current).unwrap_or(0)
        }
        let motion = settings(cx).motion;
        match self {
            Self::Insert => at(motion.insert),
            Self::Fold => at(motion.fold),
            Self::Status => at(motion.status),
            Self::Column => at(motion.column),
        }
    }

    /// What VoiceOver says of the preview: a picture of the choice, not something to operate.
    fn spoken(self) -> &'static str {
        text(match self {
            Self::Insert => SETTINGS_MOTION_INSERT_PREVIEW_LABEL,
            Self::Fold => SETTINGS_MOTION_FOLD_PREVIEW_LABEL,
            Self::Status => SETTINGS_MOTION_STATUS_PREVIEW_LABEL,
            Self::Column => SETTINGS_MOTION_COLUMN_PREVIEW_LABEL,
        })
    }

    fn id(self) -> &'static str {
        match self {
            Self::Insert => "motion-insert",
            Self::Fold => "motion-fold",
            Self::Status => "motion-status",
            Self::Column => "motion-column",
        }
    }

    /// The stage at `t`: its parts where they are then.
    fn stage(self, ui: UiColors, t: f32, cx: &App) -> Div {
        match self {
            Self::Insert => insert::stage(ui, t, cx),
            Self::Fold => fold::stage(ui, t, cx),
            Self::Status => status::stage(ui, t, cx),
            Self::Column => column::stage(ui, t, cx),
        }
    }
}

/// One preview: a scene, its frame and the clock that loops it.
pub(in crate::settings_window) struct MotionPreview {
    scene: Scene,
    /// The option shown (its place among the options) and when its loop began.
    began: Option<(usize, Instant)>,
    /// Redraws the preview between frames; dropped with it.
    _clock: Task<()>,
    /// A choice changed, or the theme did: draw again.
    _settings: Subscription,
}

impl MotionPreview {
    pub(in crate::settings_window) fn new(scene: Scene, cx: &mut Context<Self>) -> Self {
        let clock = cx.spawn(async move |this, cx| {
            loop {
                let still = cx.update(|cx| cx.reduce_motion());
                let wait = if still { STILL_CHECK } else { Duration::from_secs_f32(1.0 / ANIMATION_FPS) };
                cx.background_executor().timer(wait).await;
                if this
                    .update(cx, |_, cx| {
                        if !still {
                            cx.notify()
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            scene,
            began: None,
            _clock: clock,
            _settings: cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
        }
    }

    /// How far into its loop the option shown is: 0 when it was just picked, and always under Reduce
    /// Motion, which holds the picture before anything moves.
    fn time(&mut self, shown: usize, still: bool) -> f32 {
        if still {
            self.began = None;
            return 0.0;
        }
        let began = match self.began {
            Some((option, at)) if option == shown => at,
            _ => {
                let now = Instant::now();
                self.began = Some((shown, now));
                now
            }
        };
        (began.elapsed().as_secs_f32() / LOOP.as_secs_f32()).fract()
    }
}

impl Render for MotionPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let t = self.time(self.scene.shown(cx), cx.reduce_motion());
        div()
            .id(self.scene.id())
            .role(Role::Group)
            .aria_label(self.scene.spoken())
            .child(style::preview_frame(ui).child(self.scene.stage(ui, t, cx)))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use gpui_kit::{AppContext as _, IntoElement as _, TestAppContext};

    use super::{MotionPreview, Scene, Spot, span};

    #[test]
    fn a_span_of_the_loop_is_its_start_and_the_share_its_duration_takes() {
        let moved = span(0.25, std::time::Duration::from_secs(1));
        assert_eq!((moved.start, moved.end), (0.25, 0.5));
    }

    #[test]
    fn a_part_that_shows_nothing_is_not_built() {
        let built = std::cell::Cell::new(0);
        let build = || {
            built.set(built.get() + 1);
            gpui_kit::div().into_any_element()
        };
        assert!(Spot::row(0.0, 48.0).showing(0.0).put(build).is_none());
        assert_eq!(built.get(), 0, "not even built");
        assert!(Spot::row(0.0, 48.0).showing(0.5).put(build).is_some());
        assert_eq!(built.get(), 1);
    }

    #[gpui_kit::test]
    fn another_option_starts_the_loop_again_and_the_same_one_goes_on(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| MotionPreview::new(Scene::Fold, cx));
        let (first, again, other, still) = preview.update(cx, |preview, _| {
            let first = preview.time(1, false);
            std::thread::sleep(std::time::Duration::from_millis(30));
            let again = preview.time(1, false);
            let other = preview.time(2, false);
            let still = preview.time(2, true);
            (first, again, other, still)
        });
        assert!(first < 0.001, "just picked: at the start");
        assert!(again > first && again < 1.0, "the same option goes on: {again}");
        assert!(other < 0.001 && other < again, "another option starts over");
        assert_eq!(still, 0.0, "Reduce Motion holds the first picture");
        let began: Option<Instant> = preview.read_with(cx, |preview, _| preview.began.map(|(_, at)| at));
        assert!(began.is_none(), "and forgets the loop, so it starts over when motion is back");
    }
}
