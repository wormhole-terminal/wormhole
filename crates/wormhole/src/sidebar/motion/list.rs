//! The terminal list's motion: what its lines do when the list changes. A line that changed place
//! (pinned to the top, unpinned, dropped somewhere, a line above it coming or going) starts from where
//! it was and springs to where it is; a line that comes (a terminal opening, a group's rows opening) and
//! one that goes (closing, folding) play the pose the person's choice gives them (`pose`, from core's
//! `view::motion::pose`), the leaving ones kept drawn until it is over (`leaving`).
//!
//! Core works out who moves and how far (`sidebar::motion`) and `plan` decides what each line plays;
//! this keeps it, wraps what moves in an animation and ends it with a timer, so a list at rest costs a
//! comparison of its keys per frame and nothing else. A line's spring is its own (kept by GPUI under the line's key),
//! so a second change while it moves sets it going from where it is and how fast, not from the start.
//! Reduce Motion cuts all of it. A row being dragged is not the list's to animate: while it is held it is out of the list
//! when a gap is open for it, and the lines step aside for it (`room`), on the same springs.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::SpringSpec;
use wormhole_core::view::motion::pose::{Phase, Room};
use wormhole_core::view::sidebar::Listing;
use wormhole_core::view::sidebar::motion::Drawn;

use super::fold::{FoldNote, Summaries};
use super::join::Join;
use super::leaving::{Leavers, Snapshot};
use super::pose::{self, Natural, RowMotion};
use super::room::{self, DragRoom};
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style::motion::spring;

/// A spring is at rest within this many points of its place, which the eye cannot tell from the place.
const REST_PX: f32 = 0.25;

/// A line of the list, by what it is rather than where.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::sidebar) enum ItemKey {
    Row(i64),
    Band(String),
}

/// A line that comes: how, and the change that brought it (whose timer ends it).
pub(super) struct Arrival {
    pub(super) motion: RowMotion,
    pub(super) natural: Natural,
    pub(super) batch: u64,
}

/// The lines that spring to a new place, from and to where their top is in the content, and the
/// spring they run on, from the change that last moved them.
pub(super) struct Moving {
    pub(super) batch: u64,
    pub(super) spring: SpringSpec,
    pub(super) lines: HashMap<ItemKey, (f32, f32)>,
}

/// What the list was last drawn as, and the motion under way.
#[derive(Default)]
pub(in crate::sidebar) struct ListMotion {
    /// The lines of the listing, as last drawn.
    pub(super) keys: Vec<ItemKey>,
    /// Every line as last drawn, the leaving rows among them: the children of the list, in order.
    pub(super) drawn: Vec<ItemKey>,
    /// Where the first line starts below the top of the content, as measured at rest.
    pub(super) pad: Option<f32>,
    pub(super) moving: Option<Moving>,
    pub(super) entering: HashMap<ItemKey, Arrival>,
    pub(super) leaving: Leavers,
    pub(super) summaries: Summaries,
    /// The group the person just folded or opened, until the next change of the list takes it.
    pub(super) fold: Option<FoldNote>,
    /// What the previous frame was drawn from, which a row that leaves in this one is drawn from again:
    /// the window's facts as they were that frame, not as they were when the list last changed.
    pub(super) last: Option<(Rc<SidebarInput>, Listing)>,
    /// Rows left, so the list is not what was drawn although its keys are.
    pub(super) changed: bool,
    /// The spring the lines below a row that left run on to close its gap.
    pub(super) gap_spring: Option<SpringSpec>,
    /// The gap a dragged row has the lines open for it (`room`), from the first frame of the drag until its
    /// lines are at rest again.
    pub(super) room: Option<DragRoom>,
    /// The preview of a split a dragged row would make, and its bracket once it has (`join`).
    pub(in crate::sidebar) join: Join,
    pub(super) batch: u64,
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

    /// Whether the line `key` comes by opening its own height, which the lines below follow.
    pub(in crate::sidebar) fn opens_by_layout(&self, key: &ItemKey) -> bool {
        self.entering.get(key).is_some_and(|arrival| arrival.motion.opens_by_layout())
    }

    /// Whether each row that is leaving was the terminal on screen when it was last drawn.
    #[cfg(test)]
    pub(in crate::sidebar) fn leaving_selected(&self) -> Vec<(i64, bool)> {
        self.leaving.selected()
    }

    /// The group has just been folded (`folding`) or opened: the next change of the list is that.
    pub(in crate::sidebar) fn note_fold(&mut self, group: u32, folding: bool) {
        self.fold = Some(FoldNote { group, folding });
    }

    /// Keep what this frame is drawn from for the next, which may be the one a row leaves in. It is a
    /// count and a move: nothing is copied, so a list at rest pays nothing for it.
    pub(in crate::sidebar) fn remember(&mut self, input: &Rc<SidebarInput>, listing: Listing) {
        self.last = Some((input.clone(), listing));
    }

    /// What the previous frame was drawn from, as a snapshot the leaving rows share.
    pub(super) fn snapshot(&mut self) -> Option<Rc<Snapshot>> {
        self.last.take().map(|(input, listing)| Rc::new(Snapshot { input, listing }))
    }

    /// Everything stops where it is: the list is as it is, nothing slides or plays.
    pub(super) fn cut(&mut self) {
        self.moving = None;
        self.entering.clear();
        self.leaving.clear();
        self.summaries.clear();
    }

    /// The spring `key`'s line is on now, if it is on one: a line that changed place, or one that a
    /// dragged row pushes aside.
    pub(super) fn spring_of(&self, key: &ItemKey) -> Option<Spring> {
        if let Some(moving) = &self.moving
            && let Some(&(from, to)) = moving.lines.get(key)
        {
            return Some(Spring { spec: moving.spring, from, to, place: to });
        }
        self.room.as_ref().and_then(|room| room.spring_of(key))
    }
}

/// A line on a spring: where it starts (when nothing holds it yet) and where it is headed, both as places
/// in the content, and the place the layout itself puts it, which is what its offset is taken from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Spring {
    pub(super) spec: SpringSpec,
    pub(super) from: f32,
    pub(super) to: f32,
    pub(super) place: f32,
}

/// A key the animation of a line is told apart by: unique to the line and the change that set it going.
fn id_of(kind: &'static str, key: &ItemKey, batch: u64) -> (&'static str, u64) {
    let mut hasher = DefaultHasher::new();
    (key, batch).hash(&mut hasher);
    (kind, hasher.finish())
}

impl Sidebar {
    /// `line` (a line of the list) with the motion it owes: the pose of a line that comes or goes, and the
    /// spring to its place for one that moved. Not for the row held in a drag. `slot` is the box the line
    /// is laid out in, which stays where the layout puts it: the pointer finds the line there.
    pub(in crate::sidebar) fn with_list_motion<E: Styled + ParentElement>(
        &self,
        key: ItemKey,
        line: AnyElement,
        held: bool,
        slot: E,
    ) -> E {
        let slot = slot.relative().w_full();
        let motion = &self.motion;
        // A row held in a drag is out of the list while a gap is open for it (`room`).
        if let Some(lifted) = motion.room.as_ref().and_then(|room| room.is_lifted(&key)) {
            return slot.child(room::lifted(line, &key, lifted));
        }
        let flip = motion.spring_of(&key);
        let (arrival, leaving) = (motion.entering.get(&key), motion.leaving.get(&key));
        if held || (flip.is_none() && arrival.is_none() && leaving.is_none()) {
            return slot.child(line);
        }
        let line = match (arrival, leaving) {
            (Some(arrival), _) => {
                let id = id_of("list-enter", &key, arrival.batch);
                pose::animate(line, id, arrival.motion, Phase::Enter, &arrival.natural)
            }
            (None, Some(leaving)) => {
                let id = id_of("list-leave", &key, leaving.batch);
                pose::animate(line, id, leaving.motion, Phase::Exit, &leaving.natural)
            }
            (None, None) => line,
        };
        let Some(Spring { spec, from, to, place }) = flip else { return slot.child(line) };
        // The spring is kept under the line's key alone, so a change that retargets it carries on.
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        slot.child(div().relative().w_full().child(line).with_spring(
            ("list-move", hasher.finish()),
            SpringAnimation::new(spring(spec)).to(to).from(from).with_epsilon(REST_PX),
            move |line, top| line.top(px(top - place)),
        ))
    }
}

impl ListMotion {
    /// The change `batch` has played out: its lines stand plain.
    pub(super) fn retire_settled(&mut self, batch: u64) -> bool {
        let before = (self.entering.len(), self.moving.as_ref().map(|moving| moving.batch));
        self.entering.retain(|_, arrival| arrival.batch != batch);
        if self.moving.as_ref().is_some_and(|moving| moving.batch == batch) {
            self.moving = None;
        }
        let summary = self.summaries.retire(batch);
        let bracket = self.join.retire(batch);
        summary || bracket || before != (self.entering.len(), self.moving.as_ref().map(|moving| moving.batch))
    }

    /// The rows that left in change `batch` are over: they are dropped, and the lines below them close
    /// the gap they leave, on the spring their motion names.
    pub(super) fn retire_leaving(&mut self, batch: u64) -> bool {
        let Some(how) = self.leaving.retire(batch) else { return false };
        if let Room::Spring(spring) = how.room() {
            self.gap_spring = Some(spring);
        }
        self.changed = true;
        true
    }
}

/// Where the list's content starts, in the window: what a place in the content is measured from.
pub(super) fn content_top(scroll: &ScrollHandle) -> f32 {
    f32::from(scroll.bounds().origin.y + scroll.offset().y)
}

/// The lines last drawn with where each was (from the content's start);
/// `None` when what was measured isn't those lines (the list was rebuilt since, or is empty).
pub(super) fn measured(keys: &[ItemKey], scroll: &ScrollHandle) -> Option<Vec<Drawn<ItemKey>>> {
    if keys.is_empty() || scroll.children_count() < keys.len() {
        return None;
    }
    let content_top = content_top(scroll);
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
