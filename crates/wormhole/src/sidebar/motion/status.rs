//! A terminal's status changing in the list: which rows' tiles went from one status to another since the
//! frame before, and how the person's choice (`StatusMotion`) plays each. The poses are core's
//! (`view::motion::pose::status`), drawn by the tile, the row's wash and second line (`row`) and the row's
//! nudge (here): this notices the change, keeps the run until its time is up and says what the rows have
//! been through. A list at rest compares one pointer per frame (the facts it was drawn from) and nothing
//! else; Classic and Reduce Motion notice nothing, so the tile and the line change at once.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::StatusMotion;

use super::list::ItemKey;
use super::plan::GRACE;
use crate::settings::settings_in;
use crate::sidebar::row::{Frame, RowShift, Was, line_of, tile_parts};
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style::{ShiftClock, StatusGlyph, StatusShift};

/// A change being played: how, to what, and which frame's change it was (whose timer ends it).
struct Run {
    played: RowShift,
    to: StatusGlyph,
    change: u64,
}

/// The status changes under way, by terminal.
#[derive(Default)]
pub(in crate::sidebar) struct Statuses {
    runs: HashMap<i64, Run>,
    /// How many frames have had a change: tells one change's animations from the next's.
    changes: u64,
}

impl Statuses {
    /// What the row of terminal `id` is playing, if its status is changing.
    pub(in crate::sidebar) fn shift_of(&self, id: i64) -> Option<RowShift> {
        self.runs.get(&id).map(|run| run.played.clone())
    }

    /// The change that moves the row of terminal `id` aside (Nudge), and the status it is to.
    pub(in crate::sidebar) fn nudge_of(&self, id: i64) -> Option<(StatusShift, StatusGlyph)> {
        let run = self.runs.get(&id)?;
        (run.played.shift.motion == StatusMotion::Nudge).then_some((run.played.shift, run.to))
    }

    /// Nothing plays any more.
    pub(super) fn clear(&mut self) {
        self.runs.clear();
    }

    /// Whether any change is being played.
    #[cfg(test)]
    pub(in crate::sidebar) fn is_playing(&self) -> bool {
        !self.runs.is_empty()
    }

    /// The terminals whose status is changing in the list now, and how.
    #[cfg(test)]
    pub(in crate::sidebar) fn playing(&self) -> Vec<(i64, StatusMotion)> {
        let mut playing: Vec<_> = self.runs.iter().map(|(id, run)| (*id, run.played.shift.motion)).collect();
        playing.sort_by_key(|(id, _)| *id);
        playing
    }

    /// The changes of frame `change` are over: their rows stand plain. Whether any did.
    fn retire(&mut self, change: u64) -> bool {
        let before = self.runs.len();
        self.runs.retain(|_, run| run.change != change);
        self.runs.len() != before
    }

    /// Starts the change of every row that is listed (`listed`) and whose tile says another status in `now`
    /// than in `was`, played as `motion` says. Returns the frame's change number and how long the longest
    /// takes, when there is one.
    fn note(
        &mut self,
        (was, now): (&SidebarInput, &SidebarInput),
        frame: &Frame,
        motion: StatusMotion,
        listed: impl Fn(i64) -> bool,
    ) -> Option<(u64, Duration)> {
        let change = self.changes + 1;
        let mut longest = Duration::ZERO;
        for info in &now.rows {
            let Some(old) = was.rows.iter().find(|old| old.id == info.id) else { continue };
            let (from, to) = (tile_parts(old, frame).1, tile_parts(info, frame).1);
            let shift = StatusShift { motion, from, clock: ShiftClock::Playing(change) };
            if !shift.is_shown(to) || !listed(info.id) {
                continue;
            }
            // Only Line rolls reads the line the row said, which is not in the frame any more.
            let was = (motion == StatusMotion::LineRoll).then(|| line_of(old, frame)).and_then(|line| {
                Some(Was { text: SharedString::from(line.text?), tone: line.tone, mono: line.mono })
            });
            longest = longest.max(shift.duration(to));
            self.runs.insert(info.id, Run { played: RowShift { shift, was }, to, change });
        }
        (!longest.is_zero()).then(|| {
            self.changes = change;
            (change, longest)
        })
    }
}

impl Sidebar {
    /// Notice the terminals whose status changed since the frame before and start playing each. Call it once
    /// per render, before the list is planned (which takes what the last frame was drawn from), with `keys`
    /// the lines about to be drawn.
    pub(in crate::sidebar) fn plan_status_motion(
        &mut self,
        input: &Rc<SidebarInput>,
        keys: &[ItemKey],
        frame: &Frame,
        cx: &mut Context<Self>,
    ) {
        let motion = settings_in(&input.profile, cx).motion_in_force().status;
        if motion == StatusMotion::Classic || cx.reduce_motion() {
            self.motion.statuses.clear();
            return;
        }
        let Some((was, _)) = &self.motion.last else { return };
        // The facts are the ones the last frame drew: nothing can have changed.
        if Rc::ptr_eq(was, input) {
            return;
        }
        let was = was.clone();
        let listed = |id: i64| keys.contains(&ItemKey::Row(id));
        let Some((change, longest)) = self.motion.statuses.note((&was, input), frame, motion, listed) else {
            return;
        };
        self.later(longest + GRACE, cx, move |this, cx| {
            if this.motion.statuses.retire(change) {
                cx.notify();
            }
        });
    }
}

/// `line` (a line of the list) nudged aside as the change `shift` to `to` says, for as long as it plays.
/// Moved, not laid out again, so the rows round it stay where they are and the pointer finds the line in
/// its place.
#[track_caller]
pub(super) fn nudged(line: AnyElement, id: i64, shift: StatusShift, to: StatusGlyph) -> AnyElement {
    let held = div().relative().w_full().flex_none().child(line);
    shift.drive(held, ("status-nudge", id as u64), to, |held, pose| {
        held.when(pose.dx != 0.0, |held| held.left(px(pose.dx)))
    })
}
