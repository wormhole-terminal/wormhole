//! The title row's parts a person shows, in their order (`title_row`), and the four switches a
//! config from before the list said it with.

use std::borrow::Cow;

use super::Settings;
use crate::view::layout::title_parts::{TitlePart, TitleRow};

impl Settings {
    /// The title row's parts: `title_row`, or, in a config from before the list, the default row
    /// with the parts its old switches turned off left out.
    pub fn title_row(&self) -> Cow<'_, TitleRow> {
        match &self.title_row {
            Some(row) => Cow::Borrowed(row),
            None => Cow::Owned(TitleRow::from_switches(
                self.title_branch_status,
                self.title_agent_gauge,
                self.title_ports,
                self.title_pull_request,
            )),
        }
    }

    /// Whether the title row shows `part` (when it has something to show and the room).
    pub fn title_shows(&self, part: TitlePart) -> bool {
        self.title_row().shows(part)
    }

    /// Show or hide one of the title row's parts (see [`TitleRow::set_shown`]).
    pub fn show_title_part(&mut self, part: TitlePart, shown: bool) {
        self.title_row_mut().set_shown(part, shown);
    }

    /// Move one of the title row's parts (see [`TitleRow::move_part`]).
    pub fn move_title_part(&mut self, from: usize, to: usize) {
        self.title_row_mut().move_part(from, to);
    }

    /// The list to change, written out from now on.
    fn title_row_mut(&mut self) -> &mut TitleRow {
        let row = self.title_row().into_owned();
        self.title_row.insert(row)
    }
}
