//! The terminal list's move animation: when a row changes place (pinned to the top, unpinned,
//! dropped somewhere, a row appearing or going), the lines that moved start from where they were
//! and slide to where they are, in [`MOTION_MOVE`] easing out. Core works out who moves and how far
//! (`sidebar::motion`); this measures the lines last drawn, wraps the moved ones in an animation
//! and ends it with a timer, so a list at rest costs a comparison of its keys per frame and nothing
//! else. Reduce Motion cuts it; the row being dragged is never animated while it is held.
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::motion::{Drawn, plan_moves};

use crate::style::motion::{EASE_OUT, MOTION_MOVE};
use crate::workspace::Workspace;

/// A move longer than this is drawn from this far away, so a terminal sent across a long list
/// still slides.
const LIMIT: f32 = 480.0;
/// A line not drawn before is about this tall until it is measured (a row; a band's header is
/// shorter, but the rows below it are what slides).
const LIKELY_ROW: f32 = 44.0;

/// A line of the list, by what it is rather than where.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum ItemKey {
    Row(i64),
    Band(String),
}

/// What the list was last drawn as, and the move under way.
#[derive(Default)]
pub(super) struct ListMotion {
    keys: Vec<ItemKey>,
    /// Where the first line starts below the top of the content, as measured at rest.
    pad: Option<f32>,
    /// The lines that slide now and how far from their place each starts.
    moving: HashMap<ItemKey, f32>,
    /// Lines that appeared, faded in meanwhile.
    entering: HashSet<ItemKey>,
    serial: u64,
    end: Option<Task<()>>,
}

impl ListMotion {
    /// How far from its place `key`'s line starts, if it is sliding.
    pub(super) fn shift(&self, key: &ItemKey) -> Option<f32> {
        self.moving.get(key).copied()
    }

    pub(super) fn is_entering(&self, key: &ItemKey) -> bool {
        self.entering.contains(key)
    }

    /// Unique to this line and this move, so a later move isn't continued from an earlier one's
    /// progress.
    pub(super) fn id(&self, key: &ItemKey) -> (&'static str, u64) {
        let mut hasher = DefaultHasher::new();
        (key, self.serial).hash(&mut hasher);
        ("list-move", hasher.finish())
    }

    pub(super) fn animation() -> Animation {
        Animation::new(MOTION_MOVE).with_easing(EASE_OUT.easing())
    }
}

impl Workspace {
    /// Notice that the list's lines are not the ones last drawn, and start the slide for those
    /// that moved. Call it once per render with the lines about to be drawn, top to bottom.
    pub(super) fn plan_list_motion(&mut self, keys: Vec<ItemKey>, cx: &mut Context<Self>) {
        let motion = &mut self.sidebar.motion;
        if motion.keys == keys {
            return;
        }
        let before = std::mem::replace(&mut motion.keys, keys);
        let scroll = &self.sidebar.scroll;
        let drawn = if cx.reduce_motion() { None } else { measured(&before, scroll) };
        let Some(drawn) = drawn else {
            self.sidebar.motion.moving.clear();
            self.sidebar.motion.entering.clear();
            return;
        };
        let motion = &mut self.sidebar.motion;
        // The first line sits at the same place at rest whatever the list holds.
        if motion.moving.is_empty() || motion.pad.is_none() {
            motion.pad = Some(drawn.first().map_or(0.0, |line| line.top));
        }
        let pad = motion.pad.unwrap_or(0.0);
        let height_of = |key: &ItemKey| match key {
            ItemKey::Row(_) => LIKELY_ROW,
            ItemKey::Band(_) => drawn
                .iter()
                .find(|line| matches!(line.key, ItemKey::Band(_)))
                .map_or(LIKELY_ROW, |line| line.height),
        };
        let now: Vec<(ItemKey, f32)> = motion.keys.iter().map(|key| (key.clone(), height_of(key))).collect();
        let plan = plan_moves(&drawn, &now, pad, LIMIT);
        if plan.moved.is_empty() && plan.entering.is_empty() {
            motion.moving.clear();
            motion.entering.clear();
            return;
        }
        motion.serial += 1;
        let serial = motion.serial;
        motion.moving = plan.moved.into_iter().collect();
        motion.entering = plan.entering.into_iter().collect();
        motion.end = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(MOTION_MOVE).await;
            this.update(cx, |this, cx| {
                if this.sidebar.motion.serial == serial {
                    this.sidebar.motion.moving.clear();
                    this.sidebar.motion.entering.clear();
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    /// `row` (a line of the list) with the slide it owes: shifted to where it was and coming back,
    /// and faded in if it is new. Not for the row held in a drag.
    pub(super) fn with_list_motion(&self, key: ItemKey, line: AnyElement, held: bool) -> AnyElement {
        let motion = &self.sidebar.motion;
        let (shift, entering) = (motion.shift(&key), motion.is_entering(&key));
        let line = div().relative().w_full().child(line);
        if held || (shift.is_none() && !entering) {
            return line.into_any_element();
        }
        let from = shift.unwrap_or(0.0);
        line.with_animation(motion.id(&key), ListMotion::animation(), move |d, t| {
            d.top(px(from * (1.0 - t))).when(entering, |d| d.opacity(t))
        })
        .into_any_element()
    }
}

/// The lines last drawn with where each was (from the content's start);
/// `None` when what was measured isn't those lines (the list was rebuilt since, or is empty).
fn measured(keys: &[ItemKey], scroll: &ScrollHandle) -> Option<Vec<Drawn<ItemKey>>> {
    if keys.is_empty() || scroll.children_count() < keys.len() {
        return None;
    }
    let content_top = f32::from(scroll.bounds().origin.y + scroll.offset().y);
    keys.iter()
        .enumerate()
        .map(|(at, key)| {
            let bounds = scroll.bounds_for_item(at)?;
            Some(Drawn {
                key: key.clone(),
                top: f32::from(bounds.origin.y) - content_top,
                height: f32::from(bounds.size.height),
            })
        })
        .collect()
}
