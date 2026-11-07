//! Settings → Motion's preview: every option's scene at once, in one row of four equal cells across the
//! page's preview strip (two rows of two where a row of four has no room), each made of the app's own
//! rows, heads and tiles, looping through the thing its choice is about and named under it by the row it
//! belongs to. Each cell shows its own choice, so a pick changes only its cell; the cell of the row the
//! person touched last (the one under the pointer, the one whose menu is open, the one whose option
//! changed) wears the soft ring. Each scene is a file (`insert`, `fold`, `status`, `column`) that says where
//! its parts are at a moment of the loop; this is what they share: the view that runs the loops, the cells
//! and the spots the parts are put on.
//!
//! The scenes are drawn at the sidebar's Dense rows (Status at Regular: its second line is what one of its
//! options moves), from the top of a surface as tall as its cell, as a sidebar's list stands in its column;
//! in the two rows of two the cell is shorter than the tallest scene and cuts what does not fit at its
//! bottom. The strip is one fixed height (`preview_column::PREVIEW_H`) and nothing in it scrolls.
//!
//! The preview is a view of its own, so the clock that redraws it is its own and the Settings window's
//! page is not what ticks. It reads the time itself instead of animating built elements: a scene is a
//! function of the loop's time `t` (0.0..1.0) that builds only what shows at `t`, so a part that is
//! away (a row not there yet, the statuses it is not in) is not in the layout, the accessibility tree
//! or running an animation of its own. `t = 0` is the picture before anything moves, which is what
//! Reduce Motion holds. It exists while its page is open (`MotionStrip`), so nothing ticks while another
//! page is.
//!
//! Picking another option starts that cell's loop again from `t = 0`. Every scene draws every option from the
//! same poses the app does (core's `view::motion::pose`): the sidebar's rows, the status changes and the
//! column hiding and showing, so the preview shows exactly what the window will do.

mod column;
mod fold;
mod insert;
mod status;

use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::config::settings_index;
use wormhole_core::view::motion::MotionChoice;
use wormhole_core::view::sidebar::Density;
use wormhole_text::keys::{
    SETTINGS_MOTION_COLUMN_PREVIEW_LABEL, SETTINGS_MOTION_FOLD_PREVIEW_LABEL,
    SETTINGS_MOTION_INSERT_PREVIEW_LABEL, SETTINGS_MOTION_STATUS_PREVIEW_LABEL,
};
use wormhole_text::text;

use super::samples::{CARGO_TEST, FIX_PILE, SCRATCH, SIDEBAR_CARDS, Sample};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::preview_column::{CAPTION_ROW, FILLED_INNER_H, STAGE_INSET};
use crate::settings_window::window::page_width;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::motion::ANIMATION_FPS;
use crate::style::{self, Metrics, TextRole, TileFace};

/// The narrowest a cell gets for its rows to read: as in the other previews' cards. Four of them and
/// the gaps need the stage this wide, else the cells go two by two.
const CELL_MIN_W: f32 = 152.0;
/// The gap between cells (`gap_2`).
const GAP: f32 = 8.0;
/// The share of a miniature window the sidebar takes in the column's scene.
const SIDEBAR_SHARE: f32 = 0.5;

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

/// The sidebar's rows at its smallest density, in cards: what most scenes are drawn at, so the strip is
/// short.
fn metrics() -> Metrics {
    Metrics::of(Density::Dense, true)
}

/// The sidebar's rows with two lines (Regular), for the scene whose option moves the second line.
fn two_line_metrics() -> Metrics {
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
                .child(if self.clip < 1.0 {
                    // Cut, not squeezed: the content keeps the height it has whole.
                    div().flex_none().h(px(self.height)).child(content()).into_any_element()
                } else {
                    content()
                });
            if self.opacity < 1.0 { part.opacity(self.opacity) } else { part }.into_any_element()
        })
    }
}

/// The page's preview, as long as the page is up: the view, and the window's ear for the scene it shows.
pub(in crate::settings_window) struct MotionStrip {
    preview: Entity<MotionPreview>,
}

impl MotionStrip {
    pub(in crate::settings_window) fn new(cx: &mut Context<SettingsWindow>) -> Self {
        Self { preview: cx.new(|cx| MotionPreview::new(Scene::Insert, cx)) }
    }

    /// The scene of the row touched last.
    pub(in crate::settings_window) fn scene(&self, cx: &App) -> Scene {
        self.preview.read(cx).scene
    }

    /// The cell of `scene` is the one touched last.
    pub(in crate::settings_window) fn show(&self, scene: Scene, cx: &mut App) {
        self.preview.update(cx, |preview, cx| preview.show(scene, cx));
    }

    /// The view, to put in the strip.
    pub(in crate::settings_window) fn view(&self) -> AnyElement {
        self.preview.clone().into_any_element()
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
    const ALL: [Self; 4] = [Self::Insert, Self::Fold, Self::Status, Self::Column];

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
    pub(in crate::settings_window) fn spoken(self) -> &'static str {
        text(match self {
            Self::Insert => SETTINGS_MOTION_INSERT_PREVIEW_LABEL,
            Self::Fold => SETTINGS_MOTION_FOLD_PREVIEW_LABEL,
            Self::Status => SETTINGS_MOTION_STATUS_PREVIEW_LABEL,
            Self::Column => SETTINGS_MOTION_COLUMN_PREVIEW_LABEL,
        })
    }

    /// The id of the row the scene belongs to.
    fn id(self) -> &'static str {
        match self {
            Self::Insert => "motion-insert",
            Self::Fold => "motion-fold",
            Self::Status => "motion-status",
            Self::Column => "motion-column",
        }
    }

    /// What the row is called: the strip's head says which setting it shows.
    pub(in crate::settings_window) fn label(self) -> &'static str {
        settings_index::entry(self.id()).map_or("", |(_, _, entry)| entry.label())
    }

    /// The stage at `t`, `width` by `height` points: its parts where they are then.
    fn stage(self, ui: UiColors, t: f32, (width, height): (f32, f32), cx: &App) -> Div {
        match self {
            Self::Insert => insert::stage(ui, t, height, cx),
            Self::Fold => fold::stage(ui, t, height, cx),
            Self::Status => status::stage(ui, t, height, cx),
            Self::Column => column::stage(ui, t, (width, height), cx),
        }
    }
}

/// The preview: its four cells and the clock that loops them.
pub(in crate::settings_window) struct MotionPreview {
    /// The scene of the row touched last, whose cell wears the ring.
    scene: Scene,
    /// The option each scene's choice had when last looked at, to tell which one was changed (read at the
    /// first draw: a preview is made before the settings are looked at).
    seen: Option<[usize; 4]>,
    /// For each scene, the option it shows (its place among the options) and when its loop began.
    began: [Option<(usize, Instant)>; 4],
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
            seen: None,
            began: [None; 4],
            _clock: clock,
            _settings: cx.observe_global::<SettingsStore>(|this, cx| {
                this.follow_changes(cx);
                cx.notify()
            }),
        }
    }

    /// Show `scene`: its loop starts again from the picture before anything moves.
    fn show(&mut self, scene: Scene, cx: &mut Context<Self>) {
        if self.scene != scene {
            self.scene = scene;
            cx.notify();
        }
    }

    /// The scene of the choice that was just changed is the one to show, whoever changed it.
    fn follow_changes(&mut self, cx: &mut Context<Self>) {
        let now = Scene::ALL.map(|scene| scene.shown(cx));
        let changed = self.seen.replace(now).and_then(|before| {
            Scene::ALL
                .into_iter()
                .zip(now.into_iter().zip(before))
                .find_map(|(scene, (now, was))| (now != was).then_some(scene))
        });
        if let Some(scene) = changed {
            self.scene = scene;
        }
    }

    /// How far into its loop `scene`'s option shown is: 0 when it was just picked, and always under Reduce
    /// Motion, which holds the picture before anything moves.
    fn time(&mut self, scene: Scene, shown: usize, still: bool) -> f32 {
        let slot = &mut self.began[scene as usize];
        if still {
            *slot = None;
            return 0.0;
        }
        let began = match *slot {
            Some((was, at)) if was == shown => at,
            _ => {
                let now = Instant::now();
                *slot = Some((shown, now));
                now
            }
        };
        (began.elapsed().as_secs_f32() / LOOP.as_secs_f32()).fract()
    }
}

impl Render for MotionPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        if self.seen.is_none() {
            self.seen = Some(Scene::ALL.map(|scene| scene.shown(cx)));
        }
        let still = cx.reduce_motion();
        // The stage's own sides (`p_2` twice) come off the page's width; four cells across where each has
        // its room, else two by two, each row taking its share of the stage's height.
        let inner_w = page_width(f32::from(window.viewport_size().width)) - 2.0 * STAGE_INSET;
        let cols = if inner_w >= 4.0 * CELL_MIN_W + 3.0 * GAP { 4 } else { 2 };
        let rows = Scene::ALL.len() / cols;
        let cell_w = (inner_w - (cols - 1) as f32 * GAP) / cols as f32;
        let cell_h = (FILLED_INNER_H - (rows - 1) as f32 * GAP) / rows as f32;
        // What a cell has for its scene: its height less the caption's line and the gap above it.
        let scene_h = cell_h - CAPTION_ROW;
        let radius = ui.radius;
        let cells: Vec<AnyElement> = Scene::ALL
            .into_iter()
            .map(|scene| {
                let t = self.time(scene, scene.shown(cx), still);
                let touched = scene == self.scene;
                div()
                    .id(("motion-cell", scene as usize))
                    .flex_none()
                    .w(px(cell_w))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .role(Role::Group)
                    .aria_label(scene.spoken())
                    .child(
                        // The ring is the scene's own overlay, outside its edge: the stage clips alone.
                        div()
                            .relative()
                            .child(scene.stage(ui, t, (cell_w, scene_h), cx))
                            .when(touched, |d| d.child(style::preview_ring(ui, radius))),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_caption()
                            .text_color(if touched { ui.primary } else { ui.muted })
                            .child(scene.label()),
                    )
                    .into_any_element()
            })
            .collect();
        let mut grid = div().id("motion-preview").w_full().flex().flex_col().gap_2();
        let mut cells = cells.into_iter();
        for _ in 0..rows {
            grid = grid.child(div().flex().gap_2().children(cells.by_ref().take(cols)));
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AppContext as _, IntoElement as _, TestAppContext};

    use super::{MotionPreview, Scene, Spot};

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
            let first = preview.time(Scene::Fold, 1, false);
            std::thread::sleep(std::time::Duration::from_millis(30));
            let again = preview.time(Scene::Fold, 1, false);
            let other = preview.time(Scene::Fold, 2, false);
            let still = preview.time(Scene::Fold, 2, true);
            (first, again, other, still)
        });
        assert!(first < 0.001, "just picked: at the start");
        assert!(again > first && again < 1.0, "the same option goes on: {again}");
        assert!(other < 0.001 && other < again, "another option starts over");
        assert_eq!(still, 0.0, "Reduce Motion holds the first picture");
        let began = preview.read_with(cx, |preview, _| preview.began[Scene::Fold as usize]);
        assert!(began.is_none(), "and forgets the loop, so it starts over when motion is back");
    }

    #[gpui_kit::test]
    fn each_scene_has_a_loop_of_its_own(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| MotionPreview::new(Scene::Insert, cx));
        let (insert, fold) = preview.update(cx, |preview, _| {
            preview.time(Scene::Insert, 0, false);
            std::thread::sleep(std::time::Duration::from_millis(30));
            // Another scene's first look starts its own loop; the first one's goes on.
            (preview.time(Scene::Insert, 0, false), preview.time(Scene::Fold, 0, false))
        });
        assert!(insert > 0.0, "the first scene's loop went on: {insert}");
        assert!(fold < 0.001, "the second scene's began just now: {fold}");
    }
}
