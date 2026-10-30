//! Adapt select choices to the native picker. Stable IDs preserve arbitrary
//! values and duplicate labels when the available choices change.

use std::sync::RwLock;

use wormhole_core::view::picker::{Answer, Cancel, Provider, Query, Row};

use super::{Choice, Needle};

/// Plain, owned data safe to pass to the picker's worker thread.
#[derive(Clone, Debug)]
struct Entry {
    id: usize,
    label: String,
    detail: Option<String>,
    current: bool,
    icon: &'static str,
}

/// The caller owns the values and assigns stable IDs to them.
pub(crate) struct Choices {
    entries: RwLock<Vec<Entry>>,
}

impl Choices {
    pub(crate) fn new<V>(choices: &[Choice<V>], current: Option<usize>, icon: &'static str) -> Self {
        let result = Self { entries: RwLock::new(Vec::new()) };
        result.replace(choices, &(0..choices.len()).collect::<Vec<_>>(), current, icon);
        result
    }

    pub(crate) fn replace<V>(
        &self,
        choices: &[Choice<V>],
        ids: &[usize],
        current: Option<usize>,
        icon: &'static str,
    ) {
        assert_eq!(choices.len(), ids.len());
        let entries = choices
            .iter()
            .enumerate()
            .map(|(index, choice)| Entry {
                id: ids[index],
                label: choice.label.to_string(),
                detail: choice.detail.as_ref().map(ToString::to_string),
                current: current == Some(index),
                icon,
            })
            .collect();
        *self.entries.write().unwrap_or_else(|error| error.into_inner()) = entries;
    }
}

impl Provider for Choices {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let needle = Needle::new(&query.text);
        let entries = self.entries.read().unwrap_or_else(|error| error.into_inner());
        let rows = entries
            .iter()
            .filter_map(|entry| {
                if cancel.is_cancelled()
                    || !(needle.is_empty()
                        || needle.found_in(&entry.label)
                        || entry.detail.as_ref().is_some_and(|detail| needle.found_in(detail)))
                {
                    return None;
                }
                let mut row = Row::new(entry.id.to_string(), entry.label.clone());
                row.detail = entry.detail.clone();
                row.icon = entry.icon.into();
                row.badge = entry.current.then(|| "✓".into());
                row.insert = Some(entry.id.to_string());
                Some(row)
            })
            .collect();
        Answer { rows, ..Answer::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_keep_order_duplicates_details_and_original_indices() {
        let choices = vec![
            Choice::new("a", "All projects"),
            Choice::new("b", "App").detail("~/One/App"),
            Choice::new("c", "App").detail("~/Two/App"),
        ];
        let provider = Choices::new(&choices, Some(2), "folder");
        let query = |text: &str| {
            provider.query(&Query { text: text.into(), ..Query::default() }, &Cancel::new()).rows
        };
        let rows = query("app");
        assert_eq!(rows.iter().map(|row| row.insert.as_deref()).collect::<Vec<_>>(), [Some("1"), Some("2")]);
        assert_eq!(rows[1].badge.as_deref(), Some("✓"));
        assert_eq!(query("~/TWO")[0].insert.as_deref(), Some("2"));
        assert_eq!(query("").len(), 3);
        provider.replace(&choices[1..], &[1, 2], Some(1), "folder");
        assert_eq!(
            query("app").iter().map(|row| row.insert.as_deref()).collect::<Vec<_>>(),
            [Some("1"), Some("2")]
        );
        let cancel = Cancel::new();
        cancel.cancel();
        assert!(provider.query(&Query::default(), &cancel).rows.is_empty());
    }
}
