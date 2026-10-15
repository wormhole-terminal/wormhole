//! Pins: references to a row that outlive scrolling.
//!
//! A pin is a row's serial number and a column. The serial doesn't change when the screen scrolls
//! or the history above drops, so a pin keeps pointing at its text; once the row is older than the
//! oldest one kept it is gone and the pin says so. A reflow gives rows new numbers, so the grid
//! keeps every pin in a registry and rewrites them there; a selection, a mark or a search hit holds
//! a [`PinId`], not a number.

use crate::types::{Column, Line, Point};

use super::Grid;

/// A handle to a pin of a grid. Copyable; stale after [`Grid::unpin`](super::Grid::unpin), which
/// the grid notices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PinId {
    index: u32,
    generation: u32,
}

/// The serial number of a pin whose row is gone for good: no row ever has it.
pub(super) const DROPPED: u64 = u64::MAX;

#[derive(Clone, Debug)]
struct Slot {
    serial: u64,
    col: usize,
    generation: u32,
    live: bool,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Pins {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

impl Pins {
    pub(super) fn add(&mut self, serial: u64, col: usize) -> PinId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.serial = serial;
            slot.col = col;
            slot.live = true;
            return PinId { index, generation: slot.generation };
        }
        self.slots.push(Slot { serial, col, generation: 0, live: true });
        PinId { index: self.slots.len() as u32 - 1, generation: 0 }
    }

    fn slot(&self, pin: PinId) -> Option<&Slot> {
        self.slots.get(pin.index as usize).filter(|slot| slot.live && slot.generation == pin.generation)
    }

    pub(super) fn get(&self, pin: PinId) -> Option<(u64, usize)> {
        self.slot(pin).map(|slot| (slot.serial, slot.col))
    }

    pub(super) fn set(&mut self, pin: PinId, serial: u64, col: usize) {
        if self.slot(pin).is_some() {
            let slot = &mut self.slots[pin.index as usize];
            slot.serial = serial;
            slot.col = col;
        }
    }

    pub(super) fn remove(&mut self, pin: PinId) {
        if self.slot(pin).is_some() {
            let slot = &mut self.slots[pin.index as usize];
            slot.live = false;
            slot.generation = slot.generation.wrapping_add(1);
            self.free.push(pin.index);
        }
    }

    /// The live pins: slot index, serial, column.
    pub(super) fn live(&self) -> impl Iterator<Item = (usize, u64, usize)> + '_ {
        self.slots.iter().enumerate().filter(|(_, s)| s.live).map(|(i, s)| (i, s.serial, s.col))
    }

    /// The pins on rows from `serial` on are gone: their rows were cut off the end, and the numbers
    /// will be given to new rows, which a pin must not follow.
    pub(super) fn drop_from(&mut self, serial: u64) {
        for slot in self.slots.iter_mut().filter(|slot| slot.live && slot.serial >= serial) {
            slot.serial = DROPPED;
        }
    }

    pub(super) fn set_at(&mut self, index: usize, serial: u64, col: usize) {
        let slot = &mut self.slots[index];
        slot.serial = serial;
        slot.col = col;
    }
}

impl Grid {
    /// A reference to the row at `point` that follows its text while the screen scrolls and the
    /// width is reflowed, and says so when the row has left the history.
    pub fn pin(&mut self, point: Point) -> PinId {
        let history = self.history_len() as i32;
        let line = Line(point.line.0.clamp(-history, self.rows as i32 - 1));
        let serial = self.top_serial().saturating_add_signed(line.0 as i64);
        self.pins.add(serial, point.column.0.min(self.cols - 1))
    }

    /// Where the pin is now, `None` once its row was dropped from the history (or it was removed).
    pub fn pin_point(&self, pin: PinId) -> Option<Point> {
        let (serial, col) = self.pins.get(pin)?;
        if serial < self.first_serial() || serial >= self.end {
            return None;
        }
        let line = serial as i64 - self.top_serial() as i64;
        Some(Point::new(Line(line as i32), Column(col)))
    }

    /// Forget a pin.
    pub fn unpin(&mut self, pin: PinId) {
        self.pins.remove(pin);
    }

    /// Re-aim a pin.
    pub fn move_pin(&mut self, pin: PinId, point: Point) {
        let history = self.history_len() as i32;
        let line = Line(point.line.0.clamp(-history, self.rows as i32 - 1));
        let serial = self.top_serial().saturating_add_signed(line.0 as i64);
        self.pins.set(pin, serial, point.column.0.min(self.cols - 1));
    }
}

#[cfg(test)]
mod tests {
    use super::Pins;

    #[test]
    fn a_removed_pin_stays_removed_when_its_slot_is_reused() {
        let mut pins = Pins::default();
        let a = pins.add(10, 3);
        assert_eq!(pins.get(a), Some((10, 3)));
        pins.remove(a);
        assert_eq!(pins.get(a), None);
        let b = pins.add(20, 1);
        assert_eq!(pins.get(b), Some((20, 1)));
        assert_eq!(pins.get(a), None, "the old handle doesn't see the new pin");
    }
}
