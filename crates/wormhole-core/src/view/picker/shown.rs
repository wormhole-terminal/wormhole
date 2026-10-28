//! An answer laid out as the list shows it: section headers and rows in one line of items, then the notes.

use serde::{Deserialize, Serialize};

use super::types::{Answer, Crumb, Note, Row};

/// A line of the list.
// A row is nearly every line, so boxing it would be an allocation for each of them to save a few bytes on the
// others.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    Header(String),
    Row(Row),
    Note(Note),
}

/// Whether every provider has answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// All of them: what is listed is all there is.
    Done,
    /// Some are still out, and not for long: no word about it, and no "nothing here" yet.
    Waiting,
    /// Some are still out after the deadline: the list says so.
    Slow,
}

/// An answer as it is listed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub items: Vec<Item>,
    /// Where each row is in `items`, in the order of the rows.
    pub row_items: Vec<usize>,
    pub up: Option<String>,
    pub up_at_end: bool,
    pub crumbs: Vec<Crumb>,
}

impl Shown {
    /// `answer` listed: rows grouped by section in the sections' order (each group keeps the order it had),
    /// a header before each group that has a title, then the notes. With no rows and no note that says why,
    /// the note that there is nothing, once everything has answered; while some are still out after the
    /// deadline, the note that it is still reading.
    pub fn of(answer: Answer, reading: Reading) -> Self {
        let Answer { sections, mut rows, mut notes, up, up_at_end, crumbs } = answer;
        rows.sort_by_key(|row| row.section.min(sections.len()));
        let mut items = Vec::with_capacity(rows.len() + 3);
        let mut row_items = Vec::with_capacity(rows.len());
        let mut last = None;
        for row in rows {
            if last != Some(row.section) {
                last = Some(row.section);
                if let Some(title) = sections.get(row.section).filter(|title| !title.is_empty()) {
                    items.push(Item::Header(title.clone()));
                }
            }
            row_items.push(items.len());
            items.push(Item::Row(row));
        }
        if row_items.is_empty() && reading == Reading::Done && !notes.contains(&Note::Failed) {
            notes.insert(0, Note::Empty);
        }
        if reading == Reading::Slow {
            notes.push(Note::Reading);
        }
        items.extend(notes.into_iter().map(Item::Note));
        Self { items, row_items, up, up_at_end, crumbs }
    }

    /// The row at `row` in the order of the rows.
    pub fn row(&self, row: usize) -> Option<&Row> {
        match self.items.get(*self.row_items.get(row)?)? {
            Item::Row(row) => Some(row),
            _ => None,
        }
    }

    pub fn row_mut(&mut self, row: usize) -> Option<&mut Row> {
        let at = *self.row_items.get(row)?;
        match self.items.get_mut(at)? {
            Item::Row(row) => Some(row),
            _ => None,
        }
    }

    pub fn rows(&self) -> usize {
        self.row_items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, Reading, Shown};
    use crate::view::picker::types::{Answer, Note, Row};

    fn row(key: &str, section: usize) -> Row {
        Row { section, ..Row::new(key, key) }
    }

    #[test]
    fn a_titled_section_gets_a_header_and_the_untitled_one_none() {
        let shown = Shown::of(
            Answer {
                sections: vec![String::new(), "Recent".into()],
                rows: vec![row("a", 0), row("r", 1), row("b", 0)],
                ..Answer::default()
            },
            Reading::Done,
        );
        let line: Vec<String> = shown
            .items
            .iter()
            .map(|i| match i {
                Item::Header(t) => format!("# {t}"),
                Item::Row(r) => r.key.clone(),
                Item::Note(n) => format!("{n:?}"),
            })
            .collect();
        assert_eq!(line, ["a", "b", "# Recent", "r"]);
        assert_eq!(shown.row_items, [0, 1, 3]);
        assert_eq!(shown.row(2).map(|r| r.key.as_str()), Some("r"));
    }

    #[test]
    fn no_rows_say_so_unless_the_answer_already_said_why() {
        assert_eq!(Shown::of(Answer::default(), Reading::Done).items, [Item::Note(Note::Empty)]);
        let failed = Shown::of(Answer { notes: vec![Note::Failed], ..Answer::default() }, Reading::Done);
        assert_eq!(failed.items, [Item::Note(Note::Failed)]);
    }

    #[test]
    fn nothing_is_said_of_an_empty_list_while_a_provider_is_out_and_a_slow_one_is_named() {
        assert!(Shown::of(Answer::default(), Reading::Waiting).items.is_empty());
        assert_eq!(Shown::of(Answer::default(), Reading::Slow).items, [Item::Note(Note::Reading)]);
        let slow = Shown::of(Answer { rows: vec![row("a", 0)], ..Answer::default() }, Reading::Slow);
        assert_eq!(slow.items.last(), Some(&Item::Note(Note::Reading)));
    }

    #[test]
    fn notes_follow_the_rows() {
        let shown = Shown::of(
            Answer { rows: vec![row("a", 0)], notes: vec![Note::Limited(5)], ..Answer::default() },
            Reading::Done,
        );
        assert_eq!(shown.items.len(), 2);
        assert_eq!(shown.items[1], Item::Note(Note::Limited(5)));
    }
}
