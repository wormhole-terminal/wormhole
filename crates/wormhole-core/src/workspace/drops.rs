//! Where a drop in the list puts what was dragged, and putting it there. The shell says what the
//! pointer is over (`Over`, by id); core's rules (`view::sidebar::place`) say what that does.

use crate::view::panes::Zone;
use crate::view::sidebar::{Card, Drop, DropInput, Landing, Listing, Target, group_drop, terminal_drop};

use super::action::{CardRef, Over};
use super::event::{Event, Refusal};
use super::model::WorkspaceModel;

/// What dropping a terminal does (`view::sidebar::Drop`, by id): a shell shows the line a drag would
/// drop on from it, before the pointer is let go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPlan {
    /// It moves to `landing`. `leave_split` when it gets out of its split on the way; `with_mates`
    /// when the rest of its split moves with it.
    Move { landing: Landing, leave_split: bool, with_mates: bool },
    /// It makes a split with this terminal, or joins its split.
    Join { target: i64 },
}

impl WorkspaceModel {
    /// What dropping terminal `dragged` on `over` would do; `None` when nothing would change.
    pub fn drop_plan(&self, dragged: i64, over: Over) -> Option<DropPlan> {
        let listing = self.listing();
        let (ids, pinned, split_of) = (self.ids(), self.pinned_flags(), self.split_flags());
        let input = DropInput { listing: &listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        let drop = terminal_drop(&input, self.index_of(dragged)?, self.target_of(&listing, over)?)?;
        Some(match drop {
            Drop::Move { landing, leave_split, with_mates } => {
                DropPlan::Move { landing, leave_split, with_mates }
            }
            Drop::Join { target } => DropPlan::Join { target: ids[target] },
        })
    }

    /// `over` as the core target it means: terminals and groups by their places in the listing.
    fn target_of(&self, listing: &Listing, over: Over) -> Option<Target> {
        let band = |group: u32| listing.bands.iter().position(|band| band.group == group);
        Some(match over {
            Over::Row { id, part } => Target::Row { onto: self.index_of(id)?, part },
            Over::Head { group, part } => Target::Head { band: band(group)?, part },
            Over::After(CardRef::Group(group)) => Target::After(Card::Group(band(group)?)),
            Over::After(CardRef::Loose(id)) => Target::After(Card::Loose(self.index_of(id)?)),
            Over::After(CardRef::Split(id)) => Target::After(Card::Split(self.index_of(id)?)),
            Over::Tail => Target::Tail,
        })
    }

    /// A terminal was dropped on `over`: it moves (with the rest of its split, unless it leaves),
    /// joins the split of the terminal it is over, or leaves its own.
    pub(super) fn drop_terminal(&mut self, dragged: i64, over: Over) {
        if self.index_of(dragged).is_none() {
            return self.refuse();
        }
        match self.drop_plan(dragged, over) {
            None => self.emit(Event::Refused(Refusal::Unchanged)),
            Some(DropPlan::Move { landing, leave_split, with_mates }) => {
                if leave_split && let Some(ix) = self.index_of(dragged) {
                    self.leave_layout(ix);
                }
                self.land(dragged, landing, with_mates);
            }
            Some(DropPlan::Join { target }) => self.join_split(dragged, target, Zone::Right),
        }
    }

    /// A terminal moved to `landing`, alone.
    pub(super) fn move_terminal(&mut self, id: i64, landing: Landing) {
        if self.index_of(id).is_none() {
            return self.refuse();
        }
        self.land(id, landing, false);
    }

    /// Put what a drop landed where it landed: the terminal, with its split when `with_mates`.
    pub(super) fn land(&mut self, id: i64, landing: Landing, with_mates: bool) {
        let ids = self.ids();
        let members = if with_mates { self.with_split_mates(id) } else { vec![id] };
        for member in members {
            self.sidebar.land(&ids, member, landing);
        }
        self.rearranged();
        self.emit(Event::Moved { id });
    }

    /// Group `group` was dropped on `over`: it lands at the top level.
    pub(super) fn drop_group(&mut self, group: u32, over: Over) {
        let listing = self.listing();
        let (ids, pinned, split_of) = (self.ids(), self.pinned_flags(), self.split_flags());
        let input = DropInput { listing: &listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        let plan = listing
            .bands
            .iter()
            .position(|band| band.group == group)
            .zip(self.target_of(&listing, over))
            .and_then(|(band, target)| group_drop(&input, band, target));
        match plan {
            Some(landing) => self.move_group(group, landing),
            None => self.emit(Event::Refused(Refusal::Unchanged)),
        }
    }

    pub(super) fn move_group(&mut self, group: u32, landing: Landing) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        let ids = self.ids();
        self.sidebar.land_group(&ids, group, landing);
        self.rearranged();
    }
}
