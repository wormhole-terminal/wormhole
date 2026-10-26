//! The terminal list's motion: what its lines do when the list changes. A line that changed place
//! (pinned to the top, unpinned, dropped somewhere, a line above it coming or going) starts from where
//! it was and springs to where it is; a line that comes (a terminal opening, a group's rows opening) and
//! one that goes (closing, folding) play the pose the person's choice gives them (`row`, from core's
//! `view::motion::pose`), the leaving ones kept drawn until it is over (`leaving`).
//!
//! Core works out who moves and how far (`sidebar::motion`); this measures the lines last drawn,
//! wraps what moves in an animation and ends it with a timer, so a list at rest costs a comparison of
//! its keys per frame and nothing else. A line's spring is its own (kept by GPUI under the line's key),
//! so a second change while it moves sets it going from where it is and how fast, not from the start.
//! Reduce Motion cuts all of it; the row being dragged is never animated while it is held.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::pose::{LIST_MOVE, Phase, Room};
use wormhole_core::view::motion::{MotionChoices, SpringSpec};
use wormhole_core::view::sidebar::Listing;
use wormhole_core::view::sidebar::motion::{Drawn, Moves, plan_moves};

use super::fold::{FoldNote, Summaries};
use super::insert::{Change, changes, motions};
use super::leaving::{Leavers, Leaving, Snapshot};
use super::row::{self, Natural, RowMotion};
use crate::settings::settings_in;
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style::motion::spring;

/// A move longer than this is drawn from this far away, so a terminal sent across a long list
/// still slides.
const LIMIT: f32 = 480.0;
/// A line not drawn before is about this tall until it is measured (a row; a band's header is
/// shorter, but the rows below it are what slides).
const LIKELY_ROW: f32 = 44.0;
/// A spring is at rest for the list's purposes within this share of its distance, and under the
/// distance below, which the eye cannot tell from the place itself.
const REST: f32 = 0.005;
const REST_PX: f32 = 0.25;
/// Timers end a motion this much after it should have finished, so its last frame is drawn.
const GRACE: Duration = Duration::from_millis(16);

/// A line of the list, by what it is rather than where.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::sidebar) enum ItemKey {
    Row(i64),
    Band(String),
}

/// A line that comes: how, and the change that brought it (whose timer ends it).
struct Arrival {
    motion: RowMotion,
    natural: Natural,
    batch: u64,
}

/// The lines that spring to a new place, from and to where their top is in the content, and the
/// spring they run on, from the change that last moved them.
struct Moving {
    batch: u64,
    spring: SpringSpec,
    lines: HashMap<ItemKey, (f32, f32)>,
}

/// What the list was last drawn as, and the motion under way.
#[derive(Default)]
pub(in crate::sidebar) struct ListMotion {
    /// The lines of the listing, as last drawn.
    pub(super) keys: Vec<ItemKey>,
    /// Every line as last drawn, the leaving rows among them: the children of the list, in order.
    drawn: Vec<ItemKey>,
    /// Where the first line starts below the top of the content, as measured at rest.
    pad: Option<f32>,
    moving: Option<Moving>,
    entering: HashMap<ItemKey, Arrival>,
    pub(super) leaving: Leavers,
    pub(super) summaries: Summaries,
    /// The group the person just folded or opened, until the next change of the list takes it.
    fold: Option<FoldNote>,
    /// What the listing held when it last changed, for a row that leaves it to be drawn from.
    snapshot: Option<Rc<Snapshot>>,
    keep_snapshot: bool,
    /// Rows left, so the list is not what was drawn although its keys are.
    changed: bool,
    /// The spring the lines below a row that left run on to close its gap.
    gap_spring: Option<SpringSpec>,
    batch: u64,
}

impl ListMotion {
    /// Whether any line springs to a new place now.
    #[cfg(test)]
    pub(in crate::sidebar) fn is_moving(&self) -> bool {
        self.moving.as_ref().is_some_and(|moving| !moving.lines.is_empty())
    }

    /// Whether any line of the list is in a motion now: moving, coming or leaving.
    #[cfg(test)]
    pub(in crate::sidebar) fn is_busy(&self) -> bool {
        self.is_moving() || !self.entering.is_empty() || !self.leaving.is_empty() || self.summaries.is_up()
    }

    /// The terminals whose rows have left the list and are still drawn.
    #[cfg(test)]
    pub(in crate::sidebar) fn leaving_ids(&self) -> Vec<i64> {
        self.leaving.ids()
    }

    /// The group has just been folded (`folding`) or opened: the next change of the list is that.
    pub(in crate::sidebar) fn note_fold(&mut self, group: u32, folding: bool) {
        self.fold = Some(FoldNote { group, folding });
    }

    /// Keep what the list shows now, if it is what it was drawn as when it changed.
    pub(in crate::sidebar) fn remember(&mut self, input: &Rc<SidebarInput>, listing: Listing) {
        if std::mem::take(&mut self.keep_snapshot) {
            self.snapshot = Some(Rc::new(Snapshot { input: input.clone(), listing }));
        }
    }

    /// Everything stops where it is: the list is as it is, nothing slides or plays.
    fn cut(&mut self) {
        self.moving = None;
        self.entering.clear();
        self.leaving.clear();
        self.summaries.clear();
    }

    /// Where `key`'s line is in a spring (from, to, on which) now, if it is.
    fn flip_of(&self, key: &ItemKey) -> Option<(SpringSpec, f32, f32)> {
        let moving = self.moving.as_ref()?;
        let &(from, to) = moving.lines.get(key)?;
        Some((moving.spring, from, to))
    }
}

/// A key the animation of a line is told apart by: unique to the line and the change that set it going.
fn id_of(kind: &'static str, key: &ItemKey, batch: u64) -> (&'static str, u64) {
    let mut hasher = DefaultHasher::new();
    (key, batch).hash(&mut hasher);
    (kind, hasher.finish())
}

/// What a change of the list was: the lines that went, the lines that came, and the motion of each.
struct Turnover {
    gone: Vec<Change>,
    exits: Vec<RowMotion>,
    came: Vec<Change>,
    enters: Vec<RowMotion>,
}

/// How long the parts of a change take, for the timers that end them.
#[derive(Default)]
struct Lasting {
    /// The rows that went stay drawn this long.
    leaving: Duration,
    /// The lines that came and moved, and the summary of a folded head, take this long.
    settling: Duration,
}

/// What a plan is made of besides the list's own state.
struct Next<'a> {
    keys: &'a [ItemKey],
    listing: &'a Listing,
    fold: Option<FoldNote>,
    choices: MotionChoices,
}

impl ListMotion {
    /// Plans the change of the list to `next`: who goes, comes and moves, and how.
    fn plan(&mut self, next: &Next, scroll: &ScrollHandle, still: bool) -> Lasting {
        self.changed = false;
        self.keep_snapshot = true;
        let before = std::mem::replace(&mut self.keys, next.keys.to_vec());
        let was = if still { None } else { measured(&std::mem::take(&mut self.drawn), scroll) };
        let Some(was) = was else {
            self.cut();
            self.drawn = next.keys.to_vec();
            return Lasting::default();
        };
        let snapshot = self.snapshot.clone();
        let (gone, came) = changes(&before, snapshot.as_ref().map(|s| &s.listing), next.keys, next.listing);
        let turnover = Turnover {
            exits: motions(&gone, Phase::Exit, next.fold.as_ref(), next.choices),
            enters: motions(&came, Phase::Enter, next.fold.as_ref(), next.choices),
            gone,
            came,
        };
        self.batch += 1;
        let mut lasting = Lasting::default();
        self.keep_leaving(&turnover, &before, next.keys, snapshot, &mut lasting);
        let moves = self.plan_flips(&turnover, &was, next.keys);
        self.start_moving(moves, &turnover, &mut lasting);
        self.start_arriving(turnover, next.keys, &mut lasting);
        lasting
    }

    /// The rows that went stay drawn, where they were, for as long as their motion takes, and a folded
    /// group's head waits with its summary.
    fn keep_leaving(
        &mut self,
        turnover: &Turnover,
        before: &[ItemKey],
        now: &[ItemKey],
        snapshot: Option<Rc<Snapshot>>,
        lasting: &mut Lasting,
    ) {
        let Some(snapshot) = snapshot else { return };
        let batch = self.batch;
        let standing: HashSet<&ItemKey> = now.iter().collect();
        for (change, &how) in turnover.gone.iter().zip(&turnover.exits) {
            let stays = how.duration(Phase::Exit);
            if !matches!(change.key, ItemKey::Row(_)) || stays.is_zero() {
                continue;
            }
            let after = before[..change.at].iter().rev().find(|key| standing.contains(key)).cloned();
            let leaving = Leaving::new(change.key.clone(), after, how, change.at, snapshot.clone(), batch);
            self.leaving.push(leaving);
            lasting.leaving = lasting.leaving.max(stays);
            if let (Some(group), RowMotion::Fold(variant, slot)) = (change.group, how) {
                self.summaries.push(group, variant, slot.of, batch);
                lasting.settling = lasting.settling.max(Summaries::duration(variant, slot.of));
            }
        }
    }

    /// Where the lines that stay move to, with the leaving rows among them and a line that opens its
    /// height starting with none (the lines below follow it as it opens).
    fn plan_flips(&mut self, turnover: &Turnover, was: &[Drawn<ItemKey>], now: &[ItemKey]) -> Moves<ItemKey> {
        let by_layout = |key: &ItemKey| {
            turnover
                .came
                .iter()
                .zip(&turnover.enters)
                .any(|(came, how)| came.key == *key && how.opens_by_layout())
        };
        let likely = |key: &ItemKey| match key {
            _ if by_layout(key) => 0.0,
            ItemKey::Row(_) => LIKELY_ROW,
            ItemKey::Band(_) => was
                .iter()
                .find(|line| matches!(line.key, ItemKey::Band(_)))
                .map_or(LIKELY_ROW, |line| line.height),
        };
        let drawn = self.leaving.drawn(now);
        let lines: Vec<(ItemKey, f32)> = drawn.iter().map(|key| (key.clone(), likely(key))).collect();
        // The first line sits at the same place at rest whatever the list holds.
        if self.moving.is_none() || self.pad.is_none() {
            self.pad = Some(was.first().map_or(0.0, |line| line.top));
        }
        self.drawn = drawn;
        plan_moves(was, &lines, self.pad.unwrap_or(0.0), LIMIT)
    }

    /// The lines that moved spring to their places: on the spring a room makes for the rows (or
    /// closes after them) if there is one, else the list's own.
    fn start_moving(&mut self, moves: Moves<ItemKey>, turnover: &Turnover, lasting: &mut Lasting) {
        let room = turnover.enters.iter().chain(&turnover.exits).find_map(|how| match how.room() {
            Room::Spring(spring) => Some(spring),
            Room::Layout => None,
        });
        let spring = self.gap_spring.take().or(room).unwrap_or(LIST_MOVE);
        self.moving = (!moves.moved.is_empty()).then(|| Moving {
            batch: self.batch,
            spring,
            lines: moves.moved.iter().map(|mv| (mv.key.clone(), (mv.from, mv.to))).collect(),
        });
        if self.moving.is_some() {
            lasting.settling = lasting.settling.max(spring.settle(REST));
        }
    }

    /// The lines that came play the pose of their motion.
    fn start_arriving(&mut self, turnover: Turnover, now: &[ItemKey], lasting: &mut Lasting) {
        let standing: HashSet<&ItemKey> = now.iter().collect();
        self.entering.retain(|key, _| standing.contains(key));
        for (came, how) in turnover.came.into_iter().zip(turnover.enters) {
            lasting.settling = lasting.settling.max(how.duration(Phase::Enter));
            let arrival = Arrival { motion: how, natural: Natural::default(), batch: self.batch };
            self.entering.insert(came.key, arrival);
        }
    }
}

impl Sidebar {
    /// Notice that the list's lines are not the ones last drawn, and start the motion of those that
    /// came, went or moved. Call it once per render with the lines about to be drawn, top to bottom
    /// (`keys`, of `listing`).
    pub(in crate::sidebar) fn plan_list_motion(
        &mut self,
        keys: &[ItemKey],
        listing: &Listing,
        cx: &mut Context<Self>,
    ) {
        // A fold the person made is for the change it makes, whether or not rows come or go with it.
        let fold = self.motion.fold.take();
        if self.motion.keys == keys && !self.motion.changed {
            return;
        }
        let choices = settings_in(&self.input.profile, cx).motion_in_force();
        let next = Next { keys, listing, fold, choices };
        let lasting = self.motion.plan(&next, &self.scroll, cx.reduce_motion());
        let batch = self.motion.batch;
        // Nothing is left to see of the motion when it is over: the lines are drawn plain again.
        if !lasting.settling.is_zero() {
            self.later(lasting.settling + GRACE, cx, move |this, cx| {
                if this.motion.retire_settled(batch) {
                    cx.notify();
                }
            });
        }
        if !lasting.leaving.is_zero() {
            self.later(lasting.leaving + GRACE, cx, move |this, cx| {
                if this.motion.retire_leaving(batch) {
                    cx.notify();
                }
            });
        }
    }

    /// Runs `then` after `delay`, if the list is still there.
    fn later(
        &self,
        delay: Duration,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Context<Self>) + 'static,
    ) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, then).ok();
        })
        .detach();
    }

    /// `line` (a line of the list) with the motion it owes: the pose of a line that comes or goes,
    /// and the spring to its place for one that moved. Not for the row held in a drag.
    pub(in crate::sidebar) fn with_list_motion(
        &self,
        key: ItemKey,
        line: AnyElement,
        held: bool,
    ) -> AnyElement {
        let motion = &self.motion;
        let flip = motion.flip_of(&key);
        let (arrival, leaving) = (motion.entering.get(&key), motion.leaving.get(&key));
        if held || (flip.is_none() && arrival.is_none() && leaving.is_none()) {
            return div().relative().w_full().child(line).into_any_element();
        }
        let line = match (arrival, leaving) {
            (Some(arrival), _) => {
                let id = id_of("list-enter", &key, arrival.batch);
                row::animate(line, id, arrival.motion, Phase::Enter, &arrival.natural)
            }
            (None, Some(leaving)) => {
                let id = id_of("list-leave", &key, leaving.batch);
                row::animate(line, id, leaving.motion, Phase::Exit, &leaving.natural)
            }
            (None, None) => line,
        };
        let Some((spec, from, to)) = flip else { return line };
        // The spring is kept under the line's key alone, so a change that retargets it carries on.
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        div()
            .relative()
            .w_full()
            .child(line)
            .with_spring(
                ("list-move", hasher.finish()),
                SpringAnimation::new(spring(spec)).to(to).from(from).with_epsilon(REST_PX),
                move |line, top| line.top(px(top - to)),
            )
            .into_any_element()
    }
}

impl ListMotion {
    /// The change `batch` has played out: its lines stand plain.
    fn retire_settled(&mut self, batch: u64) -> bool {
        let before = (self.entering.len(), self.moving.as_ref().map(|moving| moving.batch));
        self.entering.retain(|_, arrival| arrival.batch != batch);
        if self.moving.as_ref().is_some_and(|moving| moving.batch == batch) {
            self.moving = None;
        }
        let summary = self.summaries.retire(batch);
        summary || before != (self.entering.len(), self.moving.as_ref().map(|moving| moving.batch))
    }

    /// The rows that left in change `batch` are over: they are dropped, and the lines below them close
    /// the gap they leave, on the spring their motion names.
    fn retire_leaving(&mut self, batch: u64) -> bool {
        let Some(how) = self.leaving.retire(batch) else { return false };
        if let Room::Spring(spring) = how.room() {
            self.gap_spring = Some(spring);
        }
        self.changed = true;
        true
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
