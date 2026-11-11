//! Setting-level search through core's `settings_index::search_visible`: the ranking, the rows a
//! hidden feature or an off switch keeps out, and the span to mark. Swift keeps no copy of the rules;
//! it looks the rows up by id in the schema it already holds.

use wormhole_core::config::settings_index::{Query, SectionId, search_visible};

use super::handle::SettingsHandle;
use crate::settings_dump::section_id;

/// A span of a row's words, in UTF-8 bytes of the English or translated text as the schema holds it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsTextSpan {
    pub start: u32,
    pub end: u32,
}

/// A row the query found, in page order with each section's best first.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsHit {
    /// The schema's section id.
    pub section: String,
    pub group: String,
    /// The row's id in the schema.
    pub entry: String,
    pub score: u32,
    /// The first query word in the label, to mark.
    pub label: Option<SettingsTextSpan>,
    /// The same in the description, when the label holds none of the words.
    pub description: Option<SettingsTextSpan>,
}

fn span(range: Option<std::ops::Range<usize>>) -> Option<SettingsTextSpan> {
    let range = range?;
    Some(SettingsTextSpan { start: u32::try_from(range.start).ok()?, end: u32::try_from(range.end).ok()? })
}

#[uniffi::export]
impl SettingsHandle {
    /// Every row `query` finds that the settings in force show. An empty query finds nothing.
    pub fn search(&self, query: String) -> Vec<SettingsHit> {
        let query = Query::new(&query);
        let hits = {
            let state = self.shared.lock();
            search_visible(&query, state.effective())
        };
        hits.into_iter()
            // The Components page is a development tool of the GPUI app; the schema has none.
            .filter(|hit| hit.section != SectionId::Components)
            .map(|hit| SettingsHit {
                section: section_id(hit.section),
                group: hit.group.title().to_string(),
                entry: hit.entry.id.to_string(),
                score: hit.score,
                label: span(hit.label_range),
                description: span(hit.description_range),
            })
            .collect()
    }
}
