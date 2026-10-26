//! The sidebar and the right panel sliding in and out, in the style the person chose (core
//! `view::motion::pose::column`: Classic, in over 250 ms easing out and out over 200 easing in, Soft spring,
//! Content after). Anything that resizes the terminal animates a clip and resizes the PTY once, never per
//! frame: a column coming in is drawn over the terminal at its full width and uncovered from its edge, and
//! joins the layout (the terminal reflowing once) when it is in; one going out leaves the layout at once (the
//! terminal reflows then) and is covered up over the terminal. The sidebar's edge carries the terminal with
//! it meanwhile (shifted, not resized), so the content moves with the column instead of jumping when it
//! lands; the right panel's edge cuts the terminal off instead (it reads from the left). Either way the
//! terminal never lies under the sliding column, which paints its own translucent surface over the blur, as
//! it does at rest. The traffic lights and the collapse button never move. Under Reduce Motion both are cuts,
//! and without the Motion feature every column is Classic.
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::view::layout::Edge;
use wormhole_core::view::motion::ColumnMotion;
use wormhole_core::view::motion::pose::column::{self, ColumnPose};

use crate::settings::{settings_in, ui_in};
use crate::style::motion::elapsed;
use crate::window::Workspace;

/// A column on its way in or out.
pub(super) struct Slide {
    edge: Edge,
    opening: bool,
    /// The column's width at rest: what it is drawn at while it slides.
    width: f32,
    /// Which slide this is, so a later one isn't ended by an earlier one's timer.
    serial: u64,
    motion: Motion,
    _end: Task<()>,
}

/// How the slide moves, which a person's choice and the moment it began decide.
#[derive(Clone, Copy)]
struct Motion {
    variant: ColumnMotion,
    /// How the column stood when it began: the rest it left, or where it was when another slide turned
    /// round (every variant but Classic, which starts from rest as it always did).
    from: ColumnPose,
    duration: Duration,
    began: Instant,
}

impl Motion {
    fn new(variant: ColumnMotion, opening: bool, from: ColumnPose, began: Instant) -> Self {
        Self { variant, from, duration: column::duration(variant, opening), began }
    }
}

/// How far a sliding column is in, frame by frame: what everything that moves with it (the column,
/// the terminal, the title row's padding) animates by, so they stay together.
#[derive(Clone, Copy)]
pub(super) struct SlideMotion {
    pub(super) edge: Edge,
    opening: bool,
    width: f32,
    serial: u64,
    motion: Motion,
}

impl SlideMotion {
    /// The slide's clock: linear over its whole length, which the pose of core reads the variant's
    /// curves and stages from.
    pub(super) fn animation(self) -> Animation {
        Animation::new(self.motion.duration)
    }

    /// The column `t` (0.0..=1.0) of the way through the slide.
    pub(super) fn pose(self, t: f32) -> ColumnPose {
        column::pose(self.motion.variant, self.opening, self.motion.from, elapsed(self.motion.duration, t))
    }

    /// How many points of the column are in at `t` of the way.
    pub(super) fn offset(self, t: f32) -> f32 {
        self.width * self.pose(t).shown
    }

    /// An animation id for `what`, unique to this slide.
    pub(super) fn id(self, what: &'static str) -> (&'static str, u64) {
        (what, self.serial)
    }
}

impl Slide {
    /// How the column stands `now`.
    fn pose_at(&self, now: Instant) -> ColumnPose {
        let elapsed = now.saturating_duration_since(self.motion.began);
        column::pose(self.motion.variant, self.opening, self.motion.from, elapsed)
    }
}

impl Workspace {
    /// `edge`'s column went from `before` to `after` points wide (a toggle): slide it, unless it
    /// neither came nor went, or motion is reduced. The Motion feature's choice says how it moves
    /// (Classic without it).
    pub(super) fn start_slide(&mut self, edge: Edge, before: f32, after: f32, cx: &mut Context<Self>) {
        if (before > 0.0) == (after > 0.0) || cx.reduce_motion() {
            return;
        }
        let opening = after > 0.0;
        let variant = settings_in(&self.profile, cx).motion_in_force().column;
        let now = cx.background_executor().now();
        // A slide toggled back while it is under way turns round from where the column is. Classic starts
        // from rest, as it always did.
        let from = match &self.slide {
            Some(slide) if variant != ColumnMotion::Classic && slide.edge == edge => slide.pose_at(now),
            _ => ColumnPose::rest(!opening),
        };
        let serial = self.slide.as_ref().map_or(0, |slide| slide.serial + 1);
        let motion = Motion::new(variant, opening, from, now);
        let duration = motion.duration;
        let end = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            this.update(cx, |this, cx| {
                if this.slide.as_ref().is_some_and(|slide| slide.serial == serial) {
                    this.slide = None;
                    cx.notify();
                }
            })
            .ok();
        });
        self.slide = Some(Slide { edge, opening, width: before.max(after), serial, motion, _end: end });
    }

    /// Whether `edge`'s column takes its place in the layout: not while it slides in over the
    /// terminal.
    pub(super) fn in_flow(&self, edge: Edge) -> bool {
        !self.slide.as_ref().is_some_and(|slide| slide.edge == edge && slide.opening)
    }

    /// The slide under way, if any.
    pub(super) fn slide_motion(&self) -> Option<SlideMotion> {
        let slide = self.slide.as_ref()?;
        Some(SlideMotion {
            edge: slide.edge,
            opening: slide.opening,
            width: slide.width,
            serial: slide.serial,
            motion: slide.motion,
        })
    }

    /// The column sliding over the terminal, at its full width with the part not yet in (or no
    /// longer in) clipped away.
    pub(super) fn render_slide(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let motion = self.slide_motion()?;
        let width = motion.width;
        let fades = motion.motion.variant == ColumnMotion::ContentAfter;
        let column = match motion.edge {
            Edge::Sidebar => self.sidebar_column(width, false, fades, window, cx),
            Edge::Panel => self.panel_column(width, fades, window, cx).into_any_element(),
        };
        // The column keeps its width inside the clip, pinned to the window's edge, so its rows
        // don't reflow as it moves. It paints its own translucent surface, as at rest: the terminal
        // never lies under it (`carry_with_slide`), so the blur shows through from the first frame.
        // Content after fades what is in the column and not its surface, so it has none and the surface is
        // drawn here, which the fade leaves alone.
        let column = if fades {
            let surface = self.column_surface(motion.edge, cx);
            div()
                .size_full()
                .child(div().absolute().size_full().bg(surface))
                .child(div().size_full().child(column).with_animation(
                    motion.id("slide-content"),
                    motion.animation(),
                    move |content, t| content.opacity(motion.pose(t).content),
                ))
                .into_any_element()
        } else {
            column
        };
        let pinned = div().absolute().top_0().bottom_0().w(px(width)).child(column);
        let pinned = match motion.edge {
            Edge::Sidebar => pinned.left_0(),
            Edge::Panel => pinned.right_0(),
        };
        let clip = div().absolute().top_0().bottom_0().overflow_hidden().child(pinned);
        let clip = match motion.edge {
            Edge::Sidebar => clip.left_0(),
            Edge::Panel => clip.right_0(),
        };
        Some(
            clip.with_animation(motion.id("slide"), motion.animation(), move |d, t| {
                d.w(px(motion.offset(t)))
            })
            .into_any_element(),
        )
    }

    /// What `edge`'s column is painted on: the translucent surface under what is in it, at rest and while it
    /// slides, so the one choice cannot drift between them.
    pub(in crate::window) fn column_surface(&self, edge: Edge, cx: &App) -> Hsla {
        let ui = ui_in(&self.profile, cx);
        match edge {
            Edge::Sidebar if self.glass_on(cx) => transparent_black(),
            Edge::Sidebar => ui.panel,
            Edge::Panel => ui.panel_right,
        }
    }

    /// The terminal's column while a column slides, kept out from under it so the sliding column
    /// shows the window's blur and not the terminal. The sidebar's edge carries it (shifted, from
    /// the window's edge to the sidebar's width coming in, back going out); the right panel's edge
    /// cuts it off (its content keeps its width, reading from the left). Only drawn there: its size
    /// (and so the PTY's) changes once, when the column joins or leaves the layout.
    pub(super) fn carry_with_slide(&self, main: Div) -> AnyElement {
        let Some(motion) = self.slide_motion() else {
            return main.into_any_element();
        };
        let at = move |t: f32| px(motion.offset(t));
        let carried = match motion.edge {
            Edge::Sidebar => main
                .with_animation(motion.id("slide-carry"), motion.animation(), move |d, t| d.left(at(t)))
                .into_any_element(),
            // The wrapper ends at the panel's edge and clips there; the negative margin gives the
            // terminal back the width the wrapper lost, so it isn't resized frame by frame.
            Edge::Panel => div()
                .flex()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .child(main.with_animation(motion.id("slide-cut-main"), motion.animation(), move |d, t| {
                    d.mr(-at(t))
                }))
                .with_animation(motion.id("slide-cut"), motion.animation(), move |d, t| d.mr(at(t)))
                .into_any_element(),
        };
        // Clipped to its place, so the part shifted past it doesn't show through a translucent
        // right panel.
        div()
            .flex()
            .flex_1()
            .min_w(px(crate::terminal_view::MIN_WIDTH))
            .h_full()
            .overflow_hidden()
            .child(carried)
            .into_any_element()
    }
}
