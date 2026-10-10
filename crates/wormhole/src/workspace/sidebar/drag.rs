//! Drag and drop in the list: a terminal moves within its folder (the whole list under
//! "No grouping"), inside its pinned or unpinned block, or joins another terminal in a split, or
//! leaves its split. The row lifts to 40 %, a ghost follows the pointer and a 2 px primary line
//! shows where it lands (full width outside any bracket, inset inside one, where it only reorders;
//! the middle of a row, which joins it in a split, is tinted instead). A pane's header dragged over
//! the list lands the same way, and dropped below the rows or on a band it takes the terminal out of
//! its split. Core works out what a drop does (`order` where a row lands, `resolve_row_drop`
//! whether it is a reorder, a join or a leave); the middle of a row in another folder's band still
//! joins it (the two stay in their own bands, joined by the chain mark), but dropping between
//! rows there or on another window does nothing (custom folders are in the backlog), except that a
//! header still leaves its split. A drag inside a bracket reorders the sidebar only, never the
//! layout.
use wormhole_core::panes::{DropError, Zone};
use wormhole_core::sidebar::order::{follow, move_in_block, regroup};
use wormhole_core::sidebar::{Part, RowDrop, resolve_row_drop};

use super::super::terminal_drag::{DragOrigin, DraggedTerminal, TerminalDrag};
use super::*;

/// What a drop would do, and the group it is worked out in: the group's terminals as listed (a
/// folder band, or the whole flat list), whether each is pinned, and the dragged one's place in it.
struct Pending {
    drop: RowDrop,
    members: Vec<usize>,
    pinned: Vec<bool>,
    from: usize,
}

/// The part of a row the pointer is over: the top quarter, the bottom quarter, or the middle half.
fn part_at(fraction: f32) -> Part {
    if fraction < 0.25 {
        Part::Top
    } else if fraction > 0.75 {
        Part::Bottom
    } else {
        Part::Middle
    }
}

impl Workspace {
    pub(super) fn with_drag(
        &self,
        row: Stateful<Div>,
        info: &RowInfo,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = info.id;
        let this = cx.weak_entity();
        let dragged =
            DraggedTerminal::new(id, DragOrigin::Row, info.name.clone(), super::rows::glyph_for(info));
        row.on_drag(dragged, move |dragged, grabbed, _, cx| {
            this.update(cx, |ws, _| ws.terminal_drag = Some(TerminalDrag::new(dragged.id, dragged.origin)))
                .ok();
            cx.new(|_| DraggedTerminal { grabbed, ..dragged.clone() })
        })
        .on_drag_move::<DraggedTerminal>(cx.listener(
            move |this, event: &DragMoveEvent<DraggedTerminal>, _, cx| {
                // Every row hears every move: only the one under the pointer counts.
                if !event.bounds.contains(&event.event.position) {
                    return;
                }
                let height = f32::from(event.bounds.size.height).max(1.0);
                let part = part_at(f32::from(event.event.position.y - event.bounds.origin.y) / height);
                if let Some(drag) = &mut this.terminal_drag
                    && (drag.onto != Some(id) || drag.part != part)
                {
                    drag.onto = Some(id);
                    drag.part = part;
                    cx.notify();
                }
            },
        ))
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, window, cx| {
            let part = this.terminal_drag.as_ref().map_or(Part::Middle, |drag| drag.part);
            this.finish_row_drop(dragged, id, part, window, cx);
        }))
    }

    /// A terminal was dropped on the `part` of the row of terminal `onto`.
    fn finish_row_drop(
        &mut self,
        dragged: &DraggedTerminal,
        onto: i64,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_drag = None;
        match self.row_drop(dragged.id, onto, part, window, cx) {
            // A header dropped where no row could take it (another folder's band) still leaves
            // its split, unless it was dropped on a terminal of that same split.
            Ok(None) if dragged.origin == DragOrigin::PaneHeader => {
                self.leave_unless_beside(dragged.id, onto, cx)
            }
            Ok(_) => {}
            Err(error) => tracing::debug!(?error, "a drop on a row was refused"),
        }
        cx.notify();
    }

    /// The list itself as a drop target: what no row took, the bands, takes the dragged terminal
    /// out of its split where it is. Its handler of the drag's move runs before the rows' (parents
    /// first), so it clears the row the pointer was over and the one under it now sets it again:
    /// over a band or beside the rows no row is marked.
    pub(super) fn with_list_drop(&self, list: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        list.on_drag_move::<DraggedTerminal>(cx.listener(
            |this, _: &DragMoveEvent<DraggedTerminal>, _, cx| {
                if let Some(drag) = &mut this.terminal_drag {
                    drag.onto = None;
                    cx.notify();
                }
            },
        ))
        .on_drop(cx.listener(|this, dragged: &DraggedTerminal, _, cx| {
            this.terminal_drag = None;
            if let Some(ix) = this.find_session(dragged.id) {
                this.leave_split(ix, cx);
                this.mark_dirty(cx);
            }
            cx.notify();
        }))
    }

    /// The space under the last row, which counts as the bottom of that row: a terminal dropped
    /// there goes to the end of its band, out of its split if it was in one. `last` is that row's
    /// terminal; none without rows.
    pub(super) fn list_tail(&self, last: Option<i64>, cx: &mut Context<Self>) -> AnyElement {
        let tail = div().id("session-tail").flex_1().min_h(px(style::SIDEBAR_TAIL_MIN_H)).w_full();
        let Some(last) = last else { return tail.into_any_element() };
        tail.on_drag_move::<DraggedTerminal>(cx.listener(
            move |this, event: &DragMoveEvent<DraggedTerminal>, _, cx| {
                if event.bounds.contains(&event.event.position)
                    && let Some(drag) = &mut this.terminal_drag
                {
                    drag.onto = Some(last);
                    drag.part = Part::Bottom;
                    cx.notify();
                }
            },
        ))
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, window, cx| {
            this.finish_row_drop(dragged, last, Part::Bottom, window, cx);
        }))
        .into_any_element()
    }

    /// A header was dropped on the row of terminal `onto`, which took nothing: its terminal leaves
    /// its split unless `onto` is in the same one.
    fn leave_unless_beside(&mut self, dragged: i64, onto: i64, cx: &mut Context<Self>) {
        let (Some(from), Some(to)) = (self.find_session(dragged), self.find_session(onto)) else { return };
        if self.sessions[from].split.is_some() && self.sessions[from].split != self.sessions[to].split {
            self.leave_split(from, cx);
            self.mark_dirty(cx);
        }
    }

    /// The group of terminals a drop of `dragged` on `onto` is worked out in: the folder band they
    /// share, or the whole flat list. `None` when they are in different bands, which does nothing.
    fn drop_group(&self, listing: &Listing, dragged: usize, onto: usize) -> Option<Vec<usize>> {
        let members: Vec<usize> = match self.sidebar.view.grouping {
            Grouping::Folder => listing.bands.get(listing.band_of(dragged)?)?.members.clone(),
            Grouping::None => listing.order().to_vec(),
        };
        (members.contains(&dragged) && members.contains(&onto)).then_some(members)
    }

    /// What dropping terminal `dragged` on the `part` of terminal `onto` would do.
    fn pending_drop(&self, listing: &Listing, dragged: i64, onto: i64, part: Part) -> Option<Pending> {
        let (dragged, onto) = (self.find_session(dragged)?, self.find_session(onto)?);
        let Some(members) = self.drop_group(listing, dragged, onto) else {
            // A row of another folder's band: there is no order to put the terminal in, but the
            // middle of the row still makes a split with it, each staying in its own band.
            let beside = self.sessions[dragged].split.is_some()
                && self.sessions[dragged].split == self.sessions[onto].split;
            return (part == Part::Middle && dragged != onto && !beside).then(|| Pending {
                drop: RowDrop::Join { target: onto, to: 0 },
                members: vec![dragged],
                pinned: vec![false],
                from: 0,
            });
        };
        let drop = resolve_row_drop(&members, &self.split_ids(), dragged, onto, part)?;
        let pinned: Vec<bool> = members.iter().map(|&ix| self.split_pinned(ix)).collect();
        let from = members.iter().position(|&ix| ix == dragged)?;
        Some(Pending { drop, members, pinned, from })
    }

    /// Where `to` (an index the dragged row is put back at once it is taken out) lies when the
    /// dragged terminal stays among its own pinned or unpinned block.
    fn clamp_to_block(pinned: &[bool], from: usize, to: usize) -> usize {
        let Some(&block) = pinned.get(from) else { return to };
        let first = pinned.iter().position(|&p| p == block).unwrap_or(0);
        let last = pinned.iter().rposition(|&p| p == block).unwrap_or(0);
        to.clamp(first, last)
    }

    /// The pending drop under the pointer, while a terminal is being dragged.
    fn pending_under_pointer(&self, listing: &Listing) -> Option<Pending> {
        let drag = self.terminal_drag.as_ref()?;
        self.pending_drop(listing, drag.id, drag.onto?, drag.part)
    }

    /// The mark of where the dragged terminal would land, on the row it lands next to: the line
    /// between two rows, or a tint over the row it would join.
    pub(super) fn drop_line(
        &self,
        info: &RowInfo,
        listing: &Listing,
        dragging: bool,
        ui: UiColors,
    ) -> Option<AnyElement> {
        if !dragging {
            return None;
        }
        let Pending { drop, members, pinned, from } = self.pending_under_pointer(listing)?;
        let to = match drop {
            RowDrop::Join { target, .. } => {
                return (target == info.ix).then(|| style::row_drop_target(ui).into_any_element());
            }
            RowDrop::Reorder { to } | RowDrop::Leave { to } => Self::clamp_to_block(&pinned, from, to),
        };
        // A reorder of a terminal that is in a split stays inside its bracket: the line is inset.
        let inside = matches!(drop, RowDrop::Reorder { .. })
            && self.sessions.get(members[from]).is_some_and(|session| session.split.is_some());
        // The line is where the row goes back in once it is taken out.
        let rest: Vec<usize> = members.iter().copied().filter(|&ix| ix != members[from]).collect();
        let (at, below) = match rest.get(to) {
            Some(&next) => (next, false),
            None => (*rest.last()?, true),
        };
        (at == info.ix).then(|| style::drop_line(ui, inside, below).into_any_element())
    }

    /// Drop terminal `dragged` on the `part` of terminal `onto`: it moves within its group, joins
    /// `onto`'s split or leaves its own (core `resolve_row_drop`). `Ok(None)` when nothing would
    /// change; an error when a join was refused (for want of room, mostly) and nothing changed.
    pub(crate) fn row_drop(
        &mut self,
        dragged: i64,
        onto: i64,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<RowDrop>, DropError> {
        let listing = self.session_listing(cx);
        let Some(Pending { drop, members, pinned, from }) = self.pending_drop(&listing, dragged, onto, part)
        else {
            return Ok(None);
        };
        match drop {
            RowDrop::Reorder { to } => self.reorder_group(&members, &pinned, from, to),
            RowDrop::Leave { to } => {
                self.leave_layout(members[from]);
                self.reorder_group(&members, &pinned, from, to);
            }
            RowDrop::Join { target, .. } => {
                let target = self.sessions[target].id;
                self.join_split(members[from], target, Zone::Right, window, cx)?;
            }
        }
        self.mark_dirty(cx);
        Ok(Some(drop))
    }

    /// Put the terminal at place `from` of the group `members` back at place `to` of what is left
    /// once it is taken out, inside its own pinned or unpinned block: only that block's places are
    /// rewritten, so the other keeps its own (unpinning a terminal puts it back where it was).
    fn reorder_group(&mut self, members: &[usize], pinned: &[bool], from: usize, to: usize) {
        let to = Self::clamp_to_block(pinned, from, to);
        let block = move_in_block(members, pinned, from, to);
        self.reorder_sessions(&regroup(self.sessions.len(), &block));
    }

    /// Put the sessions in a new manual order (`order[k]`: the session that goes to place `k`),
    /// the selection staying on its terminal.
    pub(in crate::workspace) fn reorder_sessions(&mut self, order: &[usize]) {
        let active = follow(order, self.active);
        let mut old: Vec<Option<Session>> =
            std::mem::take(&mut self.sessions).into_iter().map(Some).collect();
        self.sessions = order.iter().filter_map(|&ix| old.get_mut(ix).and_then(Option::take)).collect();
        // A bad order must not lose a terminal.
        self.sessions.extend(old.into_iter().flatten());
        self.active = active.min(self.sessions.len().saturating_sub(1));
    }
}
