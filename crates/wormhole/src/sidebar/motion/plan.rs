//! Planning the list's motion: when its lines are not the ones last drawn, who came, who went and who
//! moved, how each plays (core's poses, `view::motion::pose`) and when each is over. Everything that
//! lasts (the state, the animations, the timers' targets) is `list`'s; this decides it, once per change.

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::view::motion::MotionChoices;
use wormhole_core::view::motion::pose::{LIST_MOVE, Phase, Room};
use wormhole_core::view::sidebar::Listing;
use wormhole_core::view::sidebar::motion::{Drawn, Moves, plan_moves};

use super::fold::{FoldNote, Summaries};
use super::insert::{Change, changes, motions};
use super::leaving::{Leaving, Snapshot};
use super::list::{Arrival, ItemKey, ListMotion, Moving, measured};
use super::pose::{Natural, RowMotion};
use crate::settings::settings_in;
use crate::sidebar::Sidebar;

/// A move longer than this is drawn from this far away, so a terminal sent across a long list
/// still slides.
const LIMIT: f32 = 480.0;
/// A line not drawn before is about this tall until it is measured (a row; a band's header is
/// shorter, but the rows below it are what slides).
const LIKELY_ROW: f32 = 44.0;
/// A spring is at rest for the list's purposes within this share of its distance.
const REST: f32 = 0.005;
/// Timers end a motion this much after it should have finished, so its last frame is drawn.
const GRACE: Duration = Duration::from_millis(16);

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
        let snapshot = self.snapshot();
        let before = std::mem::replace(&mut self.keys, next.keys.to_vec());
        let was = if still { None } else { measured(&std::mem::take(&mut self.drawn), scroll) };
        let Some(was) = was else {
            self.cut();
            self.drawn = next.keys.to_vec();
            return Lasting::default();
        };
        let standing: HashSet<&ItemKey> = next.keys.iter().collect();
        // A row that came back before it was gone is the list's own again: it is not drawn twice.
        self.leaving.forget(&standing);
        let (gone, came) = changes(&before, snapshot.as_ref().map(|s| &s.listing), next.keys, next.listing);
        let turnover = Turnover {
            exits: motions(&gone, Phase::Exit, next.fold.as_ref(), next.choices),
            enters: motions(&came, Phase::Enter, next.fold.as_ref(), next.choices),
            gone,
            came,
        };
        self.batch += 1;
        let mut lasting = Lasting::default();
        self.keep_leaving(&turnover, &before, &standing, snapshot, &mut lasting);
        let moves = self.plan_flips(&turnover, &was, next.keys);
        self.start_moving(moves, &turnover, &mut lasting);
        self.start_arriving(turnover, &standing, &mut lasting);
        lasting
    }

    /// The rows that went stay drawn, where they were, for as long as their motion takes, and a folded
    /// group's head waits with its summary.
    fn keep_leaving(
        &mut self,
        turnover: &Turnover,
        before: &[ItemKey],
        standing: &HashSet<&ItemKey>,
        snapshot: Option<Rc<Snapshot>>,
        lasting: &mut Lasting,
    ) {
        let Some(snapshot) = snapshot else { return };
        let batch = self.batch;
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
    fn start_arriving(&mut self, turnover: Turnover, standing: &HashSet<&ItemKey>, lasting: &mut Lasting) {
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
}
