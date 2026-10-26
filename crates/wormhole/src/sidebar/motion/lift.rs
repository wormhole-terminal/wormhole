//! The picture that follows the pointer while a terminal is dragged by its row and the Motion feature is on:
//! the row itself, lifted out of the list as a card of its own with a shadow (GPUI has no scale, so the
//! shadow is the lift), which is what "the row travels" is. Over the middle of another row, where the drop
//! makes a split, it crossfades to the compact chip, which snaps below that row's name on a spring (`join`
//! says where) and goes back to the row's picture when the pointer leaves.
//!
//! It is a view of its own, drawn by GPUI at the pointer, that the sidebar cannot reach: so it holds what it
//! needs to draw the row (the window's facts and the listing at the moment the drag began) and a [`Snap`]
//! the sidebar says where the chip lands through.

use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::EASE_OUT;
use wormhole_core::view::motion::drag::{LIFT, chip_share};
use wormhole_core::view::sidebar::{Entry, Listing};

use super::join::Snap;
use super::list::{ItemKey, id_of};
use crate::sidebar::row::{Frame, Place, row_face};
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style::motion::SPRING_SNAP;
use crate::style::{self, GHOST_POINTER_GAP, GHOST_POINTER_RISE};

/// What the picture of a dragged row is drawn from.
#[derive(Clone)]
pub(crate) struct Lift {
    input: Rc<SidebarInput>,
    listing: Rc<Listing>,
    /// The row's place in the listing.
    at: usize,
    /// How wide the sidebar was when the row was picked up.
    width: f32,
    snap: Snap,
}

impl Sidebar {
    /// The picture for terminal `id`'s row picked up now, while the Motion feature is on.
    pub(in crate::sidebar) fn lift_of(&self, id: i64, cx: &App) -> Option<Lift> {
        if !self.motion_on(cx) {
            return None;
        }
        let listing = self.listing(cx);
        let at = listing.entries.iter().position(
            |entry| matches!(*entry, Entry::Row { session, .. } if self.input.rows[session].id == id),
        )?;
        let snap = self.snap();
        Some(Lift { input: self.input.clone(), listing: Rc::new(listing), at, width: self.width, snap })
    }
}

impl Lift {
    /// The picture, `chip` being the compact one: the row at the pointer's hand, or, over a row's middle,
    /// the chip on that row. `grabbed` is where in the row it was picked up, which GPUI draws the picture's
    /// corner by.
    pub(crate) fn picture(
        &self,
        id: i64,
        grabbed: Point<Pixels>,
        chip: Div,
        window: &mut Window,
        cx: &App,
    ) -> AnyElement {
        let (onto, at) = self.snap.target();
        let flips = self.snap.flips();
        let key = ItemKey::Row(id);
        let origin = window.mouse_position() - grabbed;
        // Where the chip lands, from the picture's own corner.
        let chip_h = self.snap.chip_h();
        let (x, y) = (f32::from(at.x - origin.x), f32::from(at.y - origin.y) - chip_h / 2.0);
        let (near_x, near_y) =
            (f32::from(grabbed.x) + GHOST_POINTER_GAP, (f32::from(grabbed.y) - GHOST_POINTER_RISE).max(0.0));
        // The first picture comes in (the lift); after, it crossfades each time the chip goes onto a row or off.
        let shares = move |t: f32| match flips {
            0 => (EASE_OUT.ease(t), 0.0),
            _ => {
                let chip = chip_share(onto, t);
                (1.0 - chip, chip)
            }
        };
        let probe = {
            let snap = self.snap.clone();
            canvas(move |bounds, _, _| snap.set_chip_h(f32::from(bounds.size.height)), |_, _, _, _| {})
                .absolute()
                .size_full()
        };
        let face =
            self.row(window, cx).map(|card| div().relative().child(card)).unwrap_or_else(div).with_animation(
                id_of("drag-face", &key, flips),
                Animation::new(LIFT),
                move |layer, t| layer.opacity(shares(t).0),
            );
        let chip = div()
            .absolute()
            .child(chip.child(probe).with_animation(
                id_of("drag-chip", &key, flips),
                Animation::new(LIFT),
                move |chip, t| chip.opacity(shares(t).1),
            ))
            .with_spring(
                id_of("drag-snap", &key, 0),
                SpringAnimation::new(SPRING_SNAP).to(onto).from(false),
                move |chip, phase| {
                    let p = phase.0;
                    chip.left(px(near_x + (x - near_x) * p)).top(px(near_y + (y - near_y) * p))
                },
            );
        div().relative().child(face).child(chip).into_any_element()
    }

    /// The row as the list drew it, as a card of its own, lifted.
    fn row(&self, window: &mut Window, cx: &App) -> Option<Div> {
        let input = &*self.input;
        let frame = Frame::new(input, self.width, false, window, cx);
        let mut place = Place::of(&self.listing, &input.rows, self.at, input.active, frame.metrics.cards)?;
        // Out of its group it is a card of its own, whatever it stood in.
        place.solo = true;
        let Entry::Row { session, .. } = self.listing.entries[self.at] else { return None };
        let info = &input.rows[session];
        let row = row_face(info, &place, &frame, None, window, cx).row;
        let width = self.width - 2.0 * frame.metrics.card_margin;
        Some(style::card_lifted(frame.ui, frame.metrics, place.selected, width, row))
    }
}
