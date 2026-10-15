//! Putting a picture on the screen at the cursor: the scrolling it needs, where the cursor goes
//! after, and the virtual placements of kitty's Unicode placeholders.

use crate::grid::NewPlacement;
use crate::types::{ImageHandle, Line};

use super::command::Command;
use super::geometry::{self, Geometry};
use super::reply::Error;
use super::upload::VirtualPlacement;
use crate::term::Terminal;

/// Most virtual placements one picture keeps.
const VIRTUALS_MAX: usize = 256;

/// Where the cursor goes after a picture is put.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Motion {
    /// Right of the picture's last row (kitty's `C=0`, iTerm2): the screen scrolls as far as the
    /// picture's rows need.
    Right,
    /// The row under the picture, at its left edge: where a Sixel leaves the cursor
    /// (DEC STD 070, sixel scrolling mode).
    Below,
    /// Where it was; nothing scrolls and the picture is cut at the bottom of the screen (kitty's
    /// `C=1`).
    Stay,
    /// The picture goes at the screen's top left whatever the cursor is, and the cursor stays; the
    /// picture is cut at the bottom (sixel display mode, DECSDM set).
    Home,
}

/// A picture to put at the cursor.
#[derive(Clone, Copy, Debug)]
pub(super) struct Placing {
    pub image: ImageHandle,
    pub image_id: u32,
    pub placement_id: u32,
    pub geometry: Geometry,
    pub z: i32,
    pub motion: Motion,
}

impl Terminal {
    /// Put the picture at the cursor as `placing` says.
    pub(super) fn put_image(&mut self, placing: Placing) {
        let Placing { image, image_id, placement_id, mut geometry, z, motion } = placing;
        if matches!(motion, Motion::Right | Motion::Below) {
            self.cursor.pending = false;
        }
        let (x0, y0) = (self.cursor.x, self.cursor.y);
        let line = match motion {
            Motion::Stay => {
                geometry = geometry.cut_rows(self.rows - y0);
                y0 as i64
            }
            Motion::Home => {
                geometry = geometry.cut_rows(self.rows);
                0
            }
            Motion::Right | Motion::Below => {
                // Without history behind it a scroll loses the lines that leave: the picture can
                // only be as tall as the part of the screen that scrolls.
                if !self.scrolls_into_history() {
                    let region = self.margins.bottom + 1 - self.margins.top;
                    geometry =
                        geometry.cut_rows(if y0 <= self.margins.bottom { region } else { self.rows - y0 });
                }
                let feeds = geometry.rows - 1 + usize::from(motion == Motion::Below);
                let mut moved = 0;
                for _ in 0..feeds {
                    let before = self.cursor.y;
                    self.index();
                    moved += usize::from(self.cursor.y > before);
                }
                // The lines the screen scrolled by: where the row the cursor started on is now.
                let top = y0 as i64 - (feeds - moved) as i64;
                // A cursor outside the scrolled region cannot make room: cut what hangs off.
                let room = (self.rows as i64 - top).max(1) as usize;
                geometry = geometry.cut_rows(room);
                top
            }
        };
        let column = if motion == Motion::Home { 0 } else { x0 };
        let new = NewPlacement {
            image,
            image_id,
            id: placement_id,
            cols: geometry.cols,
            rows: geometry.rows,
            source: geometry.source,
            offset_x: geometry.offset_x,
            offset_y: geometry.offset_y,
            extent: geometry.extent,
            z,
        };
        let line = Line(line.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
        if !self.grid.place_image(line, column, new) {
            return;
        }
        if motion == Motion::Right {
            let right = x0 + geometry.cols.min(self.cols - x0);
            if right >= self.cols {
                self.cursor.x = self.cols - 1;
                self.cursor.pending = true;
            } else {
                self.cursor.x = right;
            }
        }
    }

    /// The virtual placement (kitty's `U=1`) that placeholder cells with this picture id and
    /// placement id belong to, for sizing their tiles. A cell without a placement id uses the
    /// picture's placement without one, or its newest.
    pub fn virtual_placement(&self, image_id: u32, placement_id: u32) -> Option<VirtualPlacement> {
        let table = &self.graphics.tables[self.screen_slot()];
        let image = table.by_id(image_id).and_then(|index| table.get(index))?;
        image
            .virtuals
            .iter()
            .find(|v| v.placement_id == placement_id)
            .or_else(|| image.virtuals.last())
            .copied()
    }

    /// Whether lines that scroll off the top of the scrolling region go into the history, so that
    /// a picture may be taller than the screen.
    fn scrolls_into_history(&self) -> bool {
        !self.on_alt_screen()
            && self.margins.top == 0
            && self.margins.left == 0
            && self.margins.right == self.cols - 1
    }

    /// `a=p`, or the display half of `a=T`: put (or declare, for `U=1`) the stored picture at
    /// `index` of the screen's table.
    pub(super) fn kitty_put(&mut self, slot: usize, index: usize, command: &Command) -> Result<(), Error> {
        let table = &mut self.graphics.tables[slot];
        table.touch(index);
        let Some(image) = table.get(index) else { return Err(Error::NotFound) };
        let (handle, id, width, height) = (image.handle, image.id, image.width, image.height);
        if command.x >= width || command.y >= height {
            return Err(Error::Invalid("the part of the picture is outside it"));
        }
        let source = crate::types::SourceRect {
            x: command.x,
            y: command.y,
            width: if command.w == 0 { width - command.x } else { command.w.min(width - command.x) },
            height: if command.h == 0 { height - command.y } else { command.h.min(height - command.y) },
        };
        let cell = (u32::from(self.cell_px.0), u32::from(self.cell_px.1));
        let offset = (command.offset_x.min(cell.0 - 1), command.offset_y.min(cell.1 - 1));
        let geometry = geometry::requested(source, command.cols, command.rows, offset, cell);
        if command.unicode_placeholder {
            let virtual_placement = VirtualPlacement {
                image: handle,
                image_id: id,
                placement_id: command.placement,
                cols: geometry.cols as u32,
                rows: geometry.rows as u32,
            };
            if let Some(image) = self.graphics.tables[slot].get_mut(index) {
                if command.placement != 0 {
                    image.virtuals.retain(|v| v.placement_id != command.placement);
                }
                if image.virtuals.len() >= VIRTUALS_MAX {
                    image.virtuals.remove(0);
                }
                image.virtuals.push(virtual_placement);
            }
            return Ok(());
        }
        let motion = if command.cursor_stays { Motion::Stay } else { Motion::Right };
        self.put_image(Placing {
            image: handle,
            image_id: id,
            placement_id: command.placement,
            geometry,
            z: command.z,
            motion,
        });
        Ok(())
    }
}
