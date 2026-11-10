//! The two lists the keymap makes for Swift's overlays: the command palette's actions for a window
//! (`keymap::palette`: which, in what words and order, with which key) and the keyboard-shortcuts sheet
//! (`keymap::sheet`: sections, rows, the filter and the columns). Swift draws them and keeps no copy of the table.

use wormhole_core::keys::keymap::sheet::{self, Keys};
use wormhole_core::keys::keymap::{self, ActionId, palette};

use super::menus::MenuStateRecord;
use super::records::{KeySpecRecord, id_from_string, id_string};
use crate::folder_picker::ByteRangeRecord;

/// One action as the palette lists it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteActionRecord {
    pub id: String,
    pub label: String,
    pub key: Option<KeySpecRecord>,
    /// The Actions scope's group: `Terminal`, `View` or `App`.
    pub section: String,
}

/// What the palette lists for a window: its actions in order, and the ids it suggests with nothing typed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PaletteActionsRecord {
    pub actions: Vec<PaletteActionRecord>,
    pub suggested: Vec<String>,
}

/// The actions the palette offers for the window `state` describes (what its menus would not grey out or hide).
#[uniffi::export]
pub fn keymap_palette_actions(state: MenuStateRecord) -> PaletteActionsRecord {
    let actions = palette::actions(&state.to_core());
    let suggested = palette::suggested(&actions).iter().map(|a| id_string(a.id)).collect();
    let actions = actions
        .iter()
        .map(|a| PaletteActionRecord {
            id: id_string(a.id),
            label: a.label.clone(),
            key: a.key.as_ref().map(KeySpecRecord::from),
            section: a.section.title().to_string(),
        })
        .collect();
    PaletteActionsRecord { actions, suggested }
}

/// One row of the sheet.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SheetRowRecord {
    pub label: String,
    /// The keys to draw: each action's, or a run's first and last.
    pub keys: Vec<KeySpecRecord>,
    /// A run ("⌥⌘1 … ⌥⌘9"), not a list of keys side by side.
    pub range: bool,
    /// Where the filter matched the label, in bytes, to mark.
    pub label_hit: Option<ByteRangeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SheetSectionRecord {
    pub title: String,
    pub rows: Vec<SheetRowRecord>,
}

/// The sheet as its filter leaves it, and the counts for its footer.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SheetRecord {
    pub sections: Vec<SheetSectionRecord>,
    pub shown: u32,
    pub total: u32,
    /// The key the footer's example filter finds ("⌘W" finds Close terminal).
    pub example_key: String,
}

/// The sheet without the rows of the `hidden` actions (a feature that is off), filtered by `query`: text finds a
/// label or a key as written, a modifier glyph finds that key.
#[uniffi::export]
pub fn keymap_sheet(hidden: Vec<String>, query: String) -> SheetRecord {
    let hidden: Vec<ActionId> = hidden.iter().filter_map(|id| id_from_string(id)).collect();
    let found = sheet::filter(&sheet::sections_without(&hidden), &query);
    SheetRecord {
        sections: found
            .sections
            .iter()
            .map(|(title, rows)| SheetSectionRecord {
                title: (*title).to_string(),
                rows: rows
                    .iter()
                    .map(|(row, hit)| SheetRowRecord {
                        label: row.label.to_string(),
                        keys: row.keys.iter().map(KeySpecRecord::from).collect(),
                        range: row.style == Keys::Range,
                        label_hit: hit
                            .label
                            .as_ref()
                            .map(|r| ByteRangeRecord { start: r.start as u32, end: r.end as u32 }),
                    })
                    .collect(),
            })
            .collect(),
        shown: found.shown as u32,
        total: found.total as u32,
        example_key: keymap::glyphs(ActionId::CloseTerminal),
    }
}

/// How the sheet's sections of these `heights` go into at most `columns` columns, read down then across.
#[uniffi::export]
pub fn keymap_sheet_columns(heights: Vec<u32>, columns: u32) -> Vec<ByteRangeRecord> {
    let heights: Vec<usize> = heights.iter().map(|h| *h as usize).collect();
    sheet::columns(&heights, columns as usize)
        .into_iter()
        .map(|run| ByteRangeRecord { start: run.start as u32, end: run.end as u32 })
        .collect()
}
