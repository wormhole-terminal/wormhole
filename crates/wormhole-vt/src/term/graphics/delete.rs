//! `a=d`: deleting placements and pictures.
//!
//! Spec: kitty, "Deleting images". The letter in `d` says which placements go; a capital also
//! frees the pictures' data when nothing is placed from them any more. Without `d` the default is
//! `a`: the placements on the screen. A delete naming a picture that does not exist does
//! nothing, and is answered `OK`.
//!
//! "On the screen" means the live screen's rows, not the history; cell coordinates `x` and `y`
//! count from 1 at the screen's top left corner.

use std::collections::HashSet;

use crate::term::Terminal;
use crate::types::ImageHandle;

use super::command::{Command, Selector};
use super::reply::Error;

impl Terminal {
    pub(super) fn kitty_delete(&mut self, command: &Command) -> Result<(), Error> {
        let slot = self.screen_slot();
        let (cursor_line, cursor_col) = (self.cursor.y as i64, self.cursor.x);
        let cell = (i64::from(command.y) - 1, (command.x as usize).wrapping_sub(1));
        let table = &self.graphics.tables[slot];

        // The pictures a selector names, for the ones that name pictures.
        let named: Option<Vec<ImageHandle>> = match command.selector {
            Selector::ById => {
                if command.id == 0 {
                    return Err(Error::Invalid("no image id"));
                }
                Some(
                    table
                        .by_id(command.id)
                        .and_then(|i| table.get(i))
                        .map(|image| image.handle)
                        .into_iter()
                        .collect(),
                )
            }
            Selector::ByNumber => {
                if command.number == 0 {
                    return Err(Error::Invalid("no image number"));
                }
                Some(
                    table
                        .by_number(command.number)
                        .and_then(|i| table.get(i))
                        .map(|image| image.handle)
                        .into_iter()
                        .collect(),
                )
            }
            Selector::IdRange => Some(
                table
                    .iter()
                    .filter(|image| image.id != 0 && image.id >= command.x && image.id <= command.y)
                    .map(|image| image.handle)
                    .collect(),
            ),
            _ => None,
        };
        // A placement id narrows a delete by id or number to that placement.
        let only_placement = matches!(command.selector, Selector::ById | Selector::ByNumber)
            .then_some(command.placement)
            .filter(|&p| p != 0);

        let mut removed: Vec<ImageHandle> = Vec::new();
        let selector = command.selector;
        let z = command.z;
        let screen = if slot == self.screen_slot() { &mut self.grid } else { &mut self.other };
        match selector {
            Selector::Frames => return Ok(()),
            Selector::ById | Selector::ByNumber | Selector::IdRange => {
                let handles = named.as_deref().unwrap_or(&[]);
                screen.delete_placements(
                    |hit| {
                        handles.contains(&hit.image) && only_placement.is_none_or(|p| hit.placement_id == p)
                    },
                    &mut removed,
                );
            }
            Selector::Visible => {
                screen.delete_placements(|hit| hit.on_screen(), &mut removed);
            }
            Selector::AtCursor => {
                screen.delete_placements(|hit| hit.covers(cursor_line, cursor_col), &mut removed);
            }
            Selector::AtCell => {
                screen.delete_placements(|hit| hit.covers(cell.0, cell.1), &mut removed);
            }
            Selector::AtCellWithZ => {
                screen.delete_placements(|hit| hit.z == z && hit.covers(cell.0, cell.1), &mut removed);
            }
            Selector::InColumn => {
                screen.delete_placements(|hit| hit.on_screen() && hit.spans_column(cell.1), &mut removed);
            }
            Selector::InRow => {
                screen.delete_placements(|hit| hit.spans_line(cell.0), &mut removed);
            }
            Selector::WithZ => {
                screen.delete_placements(|hit| hit.z == z, &mut removed);
            }
        }

        // The virtual placements of the pictures a selector names go with them.
        if let Some(handles) = &named {
            for &handle in handles {
                let table = &mut self.graphics.tables[slot];
                if let Some(image) = table.find(handle).and_then(|i| table.get_mut(i)) {
                    image.virtuals.retain(|v| only_placement.is_some_and(|p| v.placement_id != p));
                }
            }
        }

        if command.free {
            let candidates: HashSet<ImageHandle> =
                removed.into_iter().chain(named.into_iter().flatten()).collect();
            for handle in candidates {
                let screen = if slot == self.screen_slot() { &self.grid } else { &self.other };
                let table = &self.graphics.tables[slot];
                let Some(index) = table.find(handle) else { continue };
                let referenced =
                    screen.is_placed(handle) || table.get(index).is_some_and(|i| !i.virtuals.is_empty());
                if !referenced {
                    self.free_image(slot, index);
                }
            }
        }
        Ok(())
    }
}
