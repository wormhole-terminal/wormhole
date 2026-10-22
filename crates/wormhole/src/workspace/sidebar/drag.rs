//! Drag and drop in the list: a terminal moves among the terminals in no group and the groups, into
//! a group or out of it, or joins another terminal in a split, or leaves its split; a group moves
//! among the others by its head. The row lifts to 40 %, a ghost follows the pointer and a 2 px
//! primary line shows where it lands (full width outside any bracket, inset inside one, where it
//! only reorders; the middle of a row, which joins it in a split, is tinted instead). A pane's
//! header dragged over the list lands the same way, and dropped below the rows it takes the
//! terminal out of its split. Core works out what a drop does (`sidebar::place`): where on the
//! arrangement it lands, or which split it joins; this reads the pointer and draws the line.
//! A drag inside a bracket reorders the sidebar only, never the layout.
use gpui_kit::*;
use wormhole_core::panes::{DropError, Zone};
use wormhole_core::sidebar::{Card, Drop, DropInput, Item, Landing, Part, Target, group_drop, terminal_drop};
use wormhole_core::sidebar::{Entry, Listing};

use super::super::terminal_drag::{DragOrigin, DraggedGroup, DraggedTerminal, Over, TerminalDrag};
use super::facts::RowInfo;
use super::motion::ItemKey;
use crate::settings::UiColors;
use crate::style;
use crate::workspace::Workspace;

/// Where a dragged thing would land, as a line on the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DropMark {
    /// The line above this one; `inside` a split's bracket, which only reorders its rows.
    Above { key: ItemKey, inside: bool },
    /// The line below it.
    Below { key: ItemKey, inside: bool },
    /// The row of this terminal would take the dragged one into its split.
    Join(i64),
}

/// The part of a line the pointer is over: the top quarter, the bottom quarter, or the middle half.
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
        let this = cx.weak_entity();
        let dragged =
            DraggedTerminal::new(info.id, DragOrigin::Row, info.name.clone(), super::rows::glyph_for(info));
        row.on_drag(dragged, move |dragged, grabbed, _, cx| {
            this.update(cx, |ws, _| ws.terminal_drag = Some(TerminalDrag::new(dragged.id, dragged.origin)))
                .ok();
            cx.new(|_| DraggedTerminal { grabbed, ..dragged.clone() })
        })
    }

    /// A group's head makes the group draggable.
    pub(super) fn with_group_drag(
        &self,
        head: Stateful<Div>,
        group: u32,
        name: SharedString,
        color: Option<Hsla>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let this = cx.weak_entity();
        let dragged = DraggedGroup { group, name, color, grabbed: Point::default() };
        head.on_drag(dragged, move |dragged, grabbed, _, cx| {
            this.update(cx, |ws, _| ws.group_drag = Some(dragged.group)).ok();
            cx.new(|_| DraggedGroup { grabbed, ..dragged.clone() })
        })
    }

    /// Terminal `id`'s line of the list as a drop target: its row and the stretch of its card round
    /// it, so the card's padding under its last row takes a drop too. `card` is the card it is a
    /// stretch of, and `gap` the gap under it when it is the card's last line: its bottom is "after the card".
    pub(super) fn with_row_drop(
        &self,
        line: Stateful<Div>,
        id: i64,
        group: Option<u32>,
        gap: Option<f32>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let over_at = move |bounds: Bounds<Pixels>, position: Point<Pixels>| {
            let height = f32::from(bounds.size.height).max(1.0);
            let y = f32::from(position.y - bounds.origin.y);
            match group {
                // The gap under a group's card puts the dragged terminal after the group.
                Some(group) if gap.is_some_and(|gap| y > height - gap) => Over::After(group),
                _ => Over::Row { id, part: part_at(y / height) },
            }
        };
        line.on_drag_move::<DraggedTerminal>(cx.listener(
            move |this, event: &DragMoveEvent<DraggedTerminal>, _, cx| {
                // Every row hears every move: only the one under the pointer counts.
                if event.bounds.contains(&event.event.position) {
                    this.set_over(Some(over_at(event.bounds, event.event.position)), cx);
                }
            },
        ))
        .on_drag_move::<DraggedGroup>(cx.listener(move |this, event: &DragMoveEvent<DraggedGroup>, _, cx| {
            if event.bounds.contains(&event.event.position) {
                this.set_over(Some(over_at(event.bounds, event.event.position)), cx);
            }
        }))
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, window, cx| {
            this.finish_terminal_drop(dragged, window, cx);
        }))
        .on_drop(cx.listener(move |this, dragged: &DraggedGroup, _, cx| {
            this.finish_group_drop(dragged, cx);
        }))
    }

    /// A group's head as a drop target; `gap` is the gap under it when it is the whole card (folded, or empty).
    pub(super) fn with_head_drop(
        &self,
        line: Stateful<Div>,
        group: u32,
        gap: Option<f32>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let over_at = move |bounds: Bounds<Pixels>, position: Point<Pixels>| {
            let height = f32::from(bounds.size.height).max(1.0);
            let y = f32::from(position.y - bounds.origin.y);
            if gap.is_some_and(|gap| y > height - gap) {
                Over::After(group)
            } else {
                Over::Head { group, part: part_at(y / height) }
            }
        };
        line.on_drag_move::<DraggedTerminal>(cx.listener(
            move |this, event: &DragMoveEvent<DraggedTerminal>, _, cx| {
                if event.bounds.contains(&event.event.position) {
                    this.set_over(Some(over_at(event.bounds, event.event.position)), cx);
                }
            },
        ))
        .on_drag_move::<DraggedGroup>(cx.listener(move |this, event: &DragMoveEvent<DraggedGroup>, _, cx| {
            if event.bounds.contains(&event.event.position) {
                this.set_over(Some(over_at(event.bounds, event.event.position)), cx);
            }
        }))
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, window, cx| {
            // The list underneath would take it out of its split instead.
            cx.stop_propagation();
            this.finish_terminal_drop(dragged, window, cx);
        }))
        .on_drop(cx.listener(move |this, dragged: &DraggedGroup, _, cx| {
            cx.stop_propagation();
            this.finish_group_drop(dragged, cx);
        }))
    }

    fn set_over(&mut self, over: Option<Over>, cx: &mut Context<Self>) {
        if self.sidebar.over != over {
            self.sidebar.over = over;
            cx.notify();
        }
    }

    /// A terminal was dropped on what the pointer was over last.
    fn finish_terminal_drop(
        &mut self,
        dragged: &DraggedTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let over = self.sidebar.over.take();
        self.terminal_drag = None;
        let result = over.map(|over| self.drop_terminal(dragged.id, over, window, cx));
        match result {
            Some(Err(error)) => tracing::debug!(?error, "a drop on a row was refused"),
            Some(Ok(Some(_))) => {}
            // A header dropped where no row could take it still leaves its split, unless it was
            // dropped on a terminal of that same split.
            Some(Ok(None)) | None if dragged.origin == DragOrigin::PaneHeader => {
                if let Some(Over::Row { id, .. }) = over {
                    self.leave_unless_beside(dragged.id, id, cx)
                } else if let Some(ix) = self.find_session(dragged.id) {
                    self.leave_split(ix, cx);
                    self.mark_dirty(cx);
                }
            }
            Some(Ok(None)) | None => {}
        }
        cx.notify();
    }

    fn finish_group_drop(&mut self, dragged: &DraggedGroup, cx: &mut Context<Self>) {
        let over = self.sidebar.over.take();
        self.group_drag = None;
        if let Some(over) = over {
            self.drop_group(dragged.group, over, cx);
        }
        cx.notify();
    }

    /// The list itself as a drop target: what no row took, the space under the last line, takes
    /// the dragged terminal out of any group and its split, to the end. Its handler of the drag's
    /// move runs before the rows' (parents first), so it clears what the pointer was over and the
    /// line under it now sets it again.
    pub(super) fn with_list_drop(&self, list: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        list.on_drag_move::<DraggedTerminal>(
            cx.listener(|this, _: &DragMoveEvent<DraggedTerminal>, _, cx| this.set_over(None, cx)),
        )
        .on_drag_move::<DraggedGroup>(
            cx.listener(|this, _: &DragMoveEvent<DraggedGroup>, _, cx| this.set_over(None, cx)),
        )
        .on_drop(cx.listener(|this, dragged: &DraggedTerminal, window, cx| {
            this.finish_terminal_drop(dragged, window, cx);
        }))
        .on_drop(cx.listener(|this, dragged: &DraggedGroup, _, cx| {
            this.finish_group_drop(dragged, cx);
        }))
    }

    /// The space under the last line, which counts as after every card: a terminal dropped there
    /// goes last, out of any group and its split.
    pub(super) fn list_tail(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("session-tail")
            .flex_1()
            .min_h(px(style::SIDEBAR_TAIL_MIN_H))
            .w_full()
            .on_drag_move::<DraggedTerminal>(cx.listener(
                |this, event: &DragMoveEvent<DraggedTerminal>, _, cx| {
                    if event.bounds.contains(&event.event.position) {
                        this.set_over(Some(Over::Tail), cx);
                    }
                },
            ))
            .on_drag_move::<DraggedGroup>(cx.listener(|this, event: &DragMoveEvent<DraggedGroup>, _, cx| {
                if event.bounds.contains(&event.event.position) {
                    this.set_over(Some(Over::Tail), cx);
                }
            }))
            .on_drop(cx.listener(|this, dragged: &DraggedTerminal, window, cx| {
                this.finish_terminal_drop(dragged, window, cx);
            }))
            .on_drop(cx.listener(|this, dragged: &DraggedGroup, _, cx| {
                this.finish_group_drop(dragged, cx);
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

    /// `over` as the core target it means: terminals and groups by their places in the listing.
    fn target_of(&self, listing: &Listing, over: Over) -> Option<Target> {
        Some(match over {
            Over::Row { id, part } => Target::Row { onto: self.find_session(id)?, part },
            Over::Head { group, part } => {
                Target::Head { band: listing.bands.iter().position(|band| band.group == group)?, part }
            }
            Over::After(group) => {
                Target::After(Card::Group(listing.bands.iter().position(|band| band.group == group)?))
            }
            Over::Tail => Target::Tail,
        })
    }

    /// What dropping terminal `dragged` on `over` would do.
    fn terminal_drop_plan(&self, listing: &Listing, dragged: i64, over: Over) -> Option<Drop> {
        let ids = self.session_ids();
        let pinned: Vec<bool> = (0..self.sessions.len()).map(|ix| self.split_pinned(ix)).collect();
        let split_of = self.split_ids();
        let input = DropInput { listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        terminal_drop(&input, self.find_session(dragged)?, self.target_of(listing, over)?)
    }

    /// What dropping group `group` on `over` would do.
    fn group_drop_plan(&self, listing: &Listing, group: u32, over: Over) -> Option<Landing> {
        let ids = self.session_ids();
        let pinned: Vec<bool> = (0..self.sessions.len()).map(|ix| self.split_pinned(ix)).collect();
        let split_of = self.split_ids();
        let input = DropInput { listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        let band = listing.bands.iter().position(|band| band.group == group)?;
        group_drop(&input, band, self.target_of(listing, over)?)
    }

    /// The line a drag in flight would drop on, if it lands somewhere.
    pub(super) fn drop_mark(&self, listing: &Listing, infos: &[RowInfo]) -> Option<DropMark> {
        let over = self.sidebar.over?;
        if let Some(dragged) = self.terminal_drag.as_ref().map(|drag| drag.id) {
            return match self.terminal_drop_plan(listing, dragged, over)? {
                Drop::Join { target } => Some(DropMark::Join(infos.get(target)?.id)),
                Drop::Move { landing, leave_split, .. } => {
                    let own = self.find_session(dragged)?;
                    let in_split = self.sessions[own].split.is_some() && !leave_split;
                    self.landing_mark(listing, infos, landing, in_split)
                }
            };
        }
        let group = self.group_drag?;
        let landing = self.group_drop_plan(listing, group, over)?;
        self.landing_mark(listing, infos, landing, false)
    }

    /// The line `landing` is drawn as: above the item it stands before, or below the last line of
    /// where it goes last.
    fn landing_mark(
        &self,
        listing: &Listing,
        infos: &[RowInfo],
        landing: Landing,
        inside: bool,
    ) -> Option<DropMark> {
        let row_key = |id: i64| ItemKey::Row(id);
        let last_key = |entry: &Entry| match *entry {
            Entry::Band { band, .. } => ItemKey::Band(listing.bands[band].key()),
            Entry::Row { session, .. } => ItemKey::Row(infos[session].id),
        };
        match landing {
            Landing::Top { before: Some(Item::Terminal(id)) } => {
                Some(DropMark::Above { key: row_key(id), inside })
            }
            Landing::Top { before: Some(Item::Group(group)) } => {
                Some(DropMark::Above { key: ItemKey::Band(wormhole_core::sidebar::group_key(group)), inside })
            }
            Landing::Top { before: None } => {
                Some(DropMark::Below { key: last_key(listing.entries.last()?), inside })
            }
            Landing::Group { before: Some(id), .. } => Some(DropMark::Above { key: row_key(id), inside }),
            Landing::Group { group, before: None } => {
                let band = listing.bands.iter().position(|band| band.group == group)?;
                let last = listing.entries.iter().rev().find(|entry| match **entry {
                    Entry::Band { band: b, .. } | Entry::Row { band: Some(b), .. } => b == band,
                    Entry::Row { band: None, .. } => false,
                })?;
                Some(DropMark::Below { key: last_key(last), inside })
            }
        }
    }

    /// The mark of the line `key`, as a drop line or a tint over it.
    pub(super) fn mark_element(
        &self,
        mark: Option<&DropMark>,
        key: &ItemKey,
        ui: UiColors,
    ) -> Option<AnyElement> {
        match mark? {
            DropMark::Above { key: at, inside } if at == key => {
                Some(style::drop_line(ui, *inside, false).into_any_element())
            }
            DropMark::Below { key: at, inside } if at == key => {
                Some(style::drop_line(ui, *inside, true).into_any_element())
            }
            DropMark::Join(id) if *key == ItemKey::Row(*id) => {
                Some(style::row_drop_target(ui).into_any_element())
            }
            _ => None,
        }
    }

    /// Drop terminal `dragged` on `over`: it moves (with the rest of its split, unless it leaves),
    /// joins `onto`'s split or leaves its own. `Ok(None)` when nothing would change; an error when
    /// a join was refused (for want of room, mostly) and nothing changed.
    fn drop_terminal(
        &mut self,
        dragged: i64,
        over: Over,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Drop>, DropError> {
        let listing = self.session_listing(cx);
        let Some(drop) = self.terminal_drop_plan(&listing, dragged, over) else { return Ok(None) };
        match drop {
            Drop::Move { landing, leave_split, with_mates } => {
                if leave_split && let Some(ix) = self.find_session(dragged) {
                    self.leave_layout(ix);
                }
                self.land_terminal(dragged, landing, with_mates, cx);
                self.sidebar.flash = Some((dragged, self.sidebar.flash.map_or(0, |(_, n)| n + 1)));
            }
            Drop::Join { target } => {
                let Some(joiner) = self.find_session(dragged) else { return Ok(None) };
                let target = self.sessions[target].id;
                self.join_split(joiner, target, Zone::Right, window, cx)?;
            }
        }
        self.mark_dirty(cx);
        Ok(Some(drop))
    }

    /// Drop group `group` on `over`.
    fn drop_group(&mut self, group: u32, over: Over, cx: &mut Context<Self>) {
        let listing = self.session_listing(cx);
        if let Some(landing) = self.group_drop_plan(&listing, group, over) {
            self.land_group(group, landing, cx);
        }
    }

    /// A drop of `dragged` on the `part` of terminal `onto`'s row (devctl, the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn row_drop(
        &mut self,
        dragged: i64,
        onto: i64,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Drop>, DropError> {
        self.drop_terminal(dragged, Over::Row { id: onto, part }, window, cx)
    }
}
