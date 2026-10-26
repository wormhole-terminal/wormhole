//! Settings → Motion's previews: under each choice, a small stage made of the app's own rows, heads
//! and tiles that loops through the thing the choice is about. Each scene is a file (`insert`,
//! `fold`, `status`, `column`); this is what they share: the frame, the loop's clock and the lanes
//! the parts of a scene ride in.
//!
//! A scene is a function of the loop's time `t` (0.0..1.0), and `t = 0` is the picture before
//! anything moves: GPUI shows the first frame under Reduce Motion, so a still preview is that one.
//! The parts are built once, up front, and a lane only moves and fades them; nothing here is
//! rebuilt from the time. The animation of a lane is keyed by the choice it shows, so picking
//! another choice starts its scene again from the top.
//!
//! Every choice draws what the app does today until the motion it names is built: the frame, the
//! picker and the restart are what these files settle, and a later phase adds a variant to the
//! scene it belongs to.

mod column;
mod fold;
mod insert;
mod status;

use std::time::Duration;

use crate::style::{Metrics, RowTone, SessionRow, StatusGlyph, TileFace};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::view::motion::{MotionChoice, Span};
use wormhole_core::view::sidebar::Density;
use wormhole_text::text;

use crate::settings::UiColors;
use crate::style::{self, TextRole};

pub(super) use column::column;
pub(super) use fold::fold;
pub(super) use insert::insert;
pub(super) use status::status;

/// The widest a preview grows: the sidebar's rows read best at about the width the sidebar has.
const STAGE_MAX_W: f32 = 480.0;

/// The length of every scene's loop: long enough to rest before and after each change.
const LOOP: Duration = Duration::from_secs(4);

/// `len` after `start`, as a stretch of the loop.
fn span(start: f32, len: Duration) -> Span {
    Span::new(start, start + len.as_secs_f32() / LOOP.as_secs_f32())
}

/// The sidebar's rows at the density most people have, in cards.
fn metrics() -> Metrics {
    Metrics::of(Density::Regular, true)
}

/// Where a part of a scene is at a moment of the loop: how far from its place it is, and how much of
/// it shows.
#[derive(Clone, Copy)]
struct Pose {
    dx: f32,
    dy: f32,
    opacity: f32,
}

impl Pose {
    /// In its place, drawn.
    const REST: Self = Self { dx: 0.0, dy: 0.0, opacity: 1.0 };

    /// In its place, drawn while `shown`.
    fn when(shown: bool) -> Self {
        Self { opacity: if shown { 1.0 } else { 0.0 }, ..Self::REST }
    }
}

/// The most lanes a scene has: what the lanes of one choice are counted in, to key them apart.
const LANES: usize = 16;

/// Which scene is on the stage and which of its choices: what keys the animations of its lanes.
#[derive(Clone, Copy)]
struct Scene {
    id: &'static str,
    /// The index of the choice in its `ALL`.
    choice: usize,
}

impl Scene {
    fn of<T: MotionChoice>(id: &'static str, current: T) -> Self {
        Self { id, choice: T::ALL.iter().position(|choice| *choice == current).unwrap_or(0) }
    }

    /// What GPUI keeps a lane's animation under: the scene, the choice shown and the lane, so another
    /// choice starts the lane's loop from its first frame.
    fn key(self, lane: usize) -> (&'static str, u64) {
        (self.id, (self.choice * LANES + lane) as u64)
    }

    /// `content` across the stage, `top` points from its top and `height` tall, posed by `pose` as the
    /// loop runs. `lane` tells the lanes of one scene apart.
    #[track_caller]
    fn lane(
        self,
        lane: usize,
        top: f32,
        height: f32,
        content: AnyElement,
        pose: impl Fn(f32) -> Pose + 'static,
    ) -> AnyElement {
        self.place(lane, None, top, height, content, pose)
    }

    /// [`Scene::lane`] for a part of the stage that has its own `width` (from the stage's left, moved
    /// by the pose), instead of running across it.
    #[track_caller]
    fn place(
        self,
        lane: usize,
        width: Option<f32>,
        top: f32,
        height: f32,
        content: AnyElement,
        pose: impl Fn(f32) -> Pose + 'static,
    ) -> AnyElement {
        div()
            .absolute()
            .left_0()
            .top(px(top))
            .h(px(height))
            .when_some(width, |lane, width| lane.w(px(width)))
            .when(width.is_none(), |lane| lane.right_0())
            .child(content)
            .with_animation(self.key(lane), style::motion::cycle(LOOP), move |lane, t| {
                let at = pose(t);
                lane.left(px(at.dx)).top(px(top + at.dy)).opacity(at.opacity)
            })
            .into_any_element()
    }
}

/// `content` where a lane would put it, but still: the parts of a scene that don't move.
#[track_caller]
fn fixed(top: f32, height: f32, content: AnyElement) -> AnyElement {
    div().absolute().left_0().right_0().top(px(top)).h(px(height)).child(content).into_any_element()
}

/// A terminal of a scene.
#[derive(Clone, Copy)]
struct Sample {
    name: &'static str,
    detail: &'static str,
    glyph: StatusGlyph,
    tone: RowTone,
    selected: bool,
    /// The agent at work in it; none draws a plain terminal.
    agent: Option<AgentId>,
}

impl Sample {
    /// The sidebar's own row for it, with its status tile. `id` tells it from the others in the
    /// window (the tile's animations are keyed by it).
    #[track_caller]
    fn row(self, id: &'static str, key: usize, ui: UiColors, cx: &App) -> AnyElement {
        let metrics = metrics();
        let tile = style::status_tile_at(
            SharedString::from(format!("{id}-tile-{key}")),
            ui,
            style::StatusTile {
                face: self.agent.map_or(TileFace::Terminal, TileFace::Agent),
                glyph: self.glyph,
                selected: self.selected,
                mini: false,
            },
            metrics.tile,
            cx,
        );
        let mut row = SessionRow::new(div().into_any_element(), self.name);
        row.selected = self.selected;
        row.tone = self.tone;
        row.tile = Some(tile);
        row.card = true;
        row.metrics = metrics;
        row.detail = Some(self.detail.into());
        style::session_row(SharedString::from(format!("{id}-row-{key}")), ui, row).into_any_element()
    }
}

/// The terminals the scenes put on their stages.
const WORKING: Sample = Sample {
    name: "Sidebar cards",  // text: sample (a made-up terminal of the preview)
    detail: "Working · 4m", // text: sample (a made-up status of the preview)
    glyph: StatusGlyph::Working,
    tone: RowTone::Plain,
    selected: true,
    agent: Some(AgentId::Claude),
};
const ASKING: Sample = Sample {
    name: "Fix pile fan-out",   // text: sample (a made-up terminal of the preview)
    detail: "Needs input · 2m", // text: sample (a made-up status of the preview)
    glyph: StatusGlyph::NeedsYou,
    tone: RowTone::Attention,
    selected: false,
    agent: Some(AgentId::Claude),
};
const RUNNING: Sample = Sample {
    name: "cargo test",               // text: sample (a made-up terminal of the preview)
    detail: "cargo test --workspace", // text: sample (a made-up command of the preview)
    glyph: StatusGlyph::Running,
    tone: RowTone::Plain,
    selected: false,
    agent: None,
};
const IDLE: Sample = Sample {
    name: "scratch", // text: sample (a made-up terminal of the preview)
    detail: "~",     // text: sample (a made-up folder of the preview)
    glyph: StatusGlyph::Idle,
    tone: RowTone::Plain,
    selected: false,
    agent: None,
};

/// A scene's stage in its frame, with the caption under it. `label` is what VoiceOver says of it: a
/// picture of the choice, not something to operate.
#[track_caller]
fn show(ui: UiColors, id: &'static str, label: &'static str, stage: Div) -> AnyElement {
    div()
        .id(id)
        .role(Role::Group)
        .aria_label(label)
        .flex()
        .flex_col()
        .gap_2()
        .child(style::preview_frame(ui).max_w(px(STAGE_MAX_W)).child(stage))
        .child(
            div()
                .text_caption()
                .text_color(ui.muted)
                .child(text(wormhole_text::keys::SETTINGS_MOTION_PREVIEW_CAPTION)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, StatusMotion};

    use super::{LANES, Scene, span};

    /// Every lane of every option of a choice has a key of its own.
    fn keys_of<T: MotionChoice>(id: &'static str) -> Vec<(&'static str, u64)> {
        T::ALL
            .iter()
            .flat_map(|choice| (0..LANES).map(move |lane| Scene::of(id, *choice).key(lane)))
            .collect()
    }

    #[test]
    fn picking_another_option_keys_the_loop_anew_and_the_same_one_keeps_it() {
        for keys in [
            keys_of::<InsertMotion>("motion-insert"),
            keys_of::<FoldMotion>("motion-fold"),
            keys_of::<StatusMotion>("motion-status"),
            keys_of::<ColumnMotion>("motion-column"),
        ] {
            assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len(), "two lanes share a key");
        }
        assert_eq!(
            Scene::of("motion-fold", FoldMotion::Drawer).key(3),
            Scene::of("motion-fold", FoldMotion::Drawer).key(3),
            "the same option on the next frame is the same animation, still running"
        );
        assert_ne!(
            Scene::of("motion-fold", FoldMotion::Drawer).key(3),
            Scene::of("motion-fold", FoldMotion::Tuck).key(3)
        );
    }

    #[test]
    fn a_span_of_the_loop_is_its_start_and_the_share_its_duration_takes() {
        let moved = span(0.25, std::time::Duration::from_secs(1));
        assert_eq!((moved.start, moved.end), (0.25, 0.5));
    }
}
