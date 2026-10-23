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
use wormhole_core::view::sidebar::{
    Card, Drop, DropInput, Item, Landing, Part, Target, group_drop, terminal_drop,
};
use wormhole_core::view::sidebar::{Entry, Listing};

use super::facts::RowInfo;
use super::input::SidebarInput;
use super::motion::ItemKey;
use super::{Sidebar, SidebarEvent};
use crate::settings::UiColors;
use crate::style;
use crate::window::sidebar::terminal_drag::{DragOrigin, DraggedGroup, DraggedTerminal, Over};

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

impl Sidebar {
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
            this.update(cx, |_, cx| {
                cx.emit(SidebarEvent::TerminalDragStarted { id: dragged.id, origin: dragged.origin })
            })
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
            this.update(cx, |_, cx| cx.emit(SidebarEvent::GroupDragStarted(dragged.group))).ok();
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
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, _, cx| {
            this.finish_terminal_drop(dragged, cx);
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
        .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, _, cx| {
            // The list underneath would take it out of its split instead.
            cx.stop_propagation();
            this.finish_terminal_drop(dragged, cx);
        }))
        .on_drop(cx.listener(move |this, dragged: &DraggedGroup, _, cx| {
            cx.stop_propagation();
            this.finish_group_drop(dragged, cx);
        }))
    }

    fn set_over(&mut self, over: Option<Over>, cx: &mut Context<Self>) {
        if self.over != over {
            self.over = over;
            cx.notify();
        }
    }

    /// A terminal was dropped on what the pointer was over last: the window does what it means
    /// (a move, a join, leaving a split).
    fn finish_terminal_drop(&mut self, dragged: &DraggedTerminal, cx: &mut Context<Self>) {
        let over = self.over.take();
        cx.emit(SidebarEvent::TerminalDropped { dragged: dragged.id, origin: dragged.origin, over });
        cx.notify();
    }

    fn finish_group_drop(&mut self, dragged: &DraggedGroup, cx: &mut Context<Self>) {
        let over = self.over.take();
        cx.emit(SidebarEvent::GroupDragEnded);
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
        .on_drop(cx.listener(|this, dragged: &DraggedTerminal, _, cx| {
            this.finish_terminal_drop(dragged, cx);
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
            .on_drop(cx.listener(|this, dragged: &DraggedTerminal, _, cx| {
                this.finish_terminal_drop(dragged, cx);
            }))
            .on_drop(cx.listener(|this, dragged: &DraggedGroup, _, cx| {
                this.finish_group_drop(dragged, cx);
            }))
            .into_any_element()
    }

    /// `over` as the core target it means: terminals and groups by their places in the listing.
    fn target_of(input: &SidebarInput, listing: &Listing, over: Over) -> Option<Target> {
        Some(match over {
            Over::Row { id, part } => Target::Row { onto: input.find(id)?, part },
            Over::Head { group, part } => {
                Target::Head { band: listing.bands.iter().position(|band| band.group == group)?, part }
            }
            Over::After(group) => {
                Target::After(Card::Group(listing.bands.iter().position(|band| band.group == group)?))
            }
            Over::Tail => Target::Tail,
        })
    }

    /// What dropping terminal `dragged` on `over` would do, of the terminals `input` says there are.
    pub(in crate::window) fn terminal_drop_plan(
        input: &SidebarInput,
        listing: &Listing,
        dragged: i64,
        over: Over,
    ) -> Option<Drop> {
        let (ids, pinned, split_of) = (input.ids(), input.pinned(), input.split_of());
        let drop = DropInput { listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        terminal_drop(&drop, input.find(dragged)?, Self::target_of(input, listing, over)?)
    }

    /// What dropping group `group` on `over` would do.
    fn group_drop_plan(input: &SidebarInput, listing: &Listing, group: u32, over: Over) -> Option<Landing> {
        let (ids, pinned, split_of) = (input.ids(), input.pinned(), input.split_of());
        let drop = DropInput { listing, ids: &ids, split_of: &split_of, pinned: &pinned };
        let band = listing.bands.iter().position(|band| band.group == group)?;
        group_drop(&drop, band, Self::target_of(input, listing, over)?)
    }

    /// Drop group `group` on `over`.
    fn drop_group(&mut self, group: u32, over: Over, cx: &mut Context<Self>) {
        let input = self.input.clone();
        let listing = self.listing_of(&input, cx);
        if let Some(landing) = Self::group_drop_plan(&input, &listing, group, over) {
            self.land_group(group, landing, cx);
        }
    }

    /// The line a drag in flight would drop on, if it lands somewhere.
    pub(super) fn drop_mark(&self, listing: &Listing, infos: &[RowInfo]) -> Option<DropMark> {
        let over = self.over?;
        let input = &self.input;
        if let Some((dragged, _)) = input.terminal_drag {
            return match Self::terminal_drop_plan(input, listing, dragged, over)? {
                Drop::Join { target } => Some(DropMark::Join(infos.get(target)?.id)),
                Drop::Move { landing, leave_split, .. } => {
                    let own = input.find(dragged)?;
                    let in_split = input.rows[own].split.is_some() && !leave_split;
                    self.landing_mark(listing, infos, landing, in_split)
                }
            };
        }
        let group = input.group_drag?;
        let landing = Self::group_drop_plan(input, listing, group, over)?;
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
            Landing::Top { before: Some(Item::Group(group)) } => Some(DropMark::Above {
                key: ItemKey::Band(wormhole_core::view::sidebar::group_key(group)),
                inside,
            }),
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
}
