//! The font catalog as data for the shared native picker.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use wormhole_core::theme::fonts::{FontSlot, FontSource, grouped_font_choices};
use wormhole_core::view::picker::{Answer, Cancel, Note, Provider, Query, Row};
use wormhole_text::{keys::settings_fonts_count, whole};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Catalog {
    pub slot: FontSlot,
    pub installed: Arc<Vec<String>>,
    pub bundled: Vec<String>,
    pub custom: Vec<String>,
    pub monospace: Option<Arc<HashSet<String>>>,
    pub symbolic: Arc<HashSet<String>>,
    pub all: bool,
    pub selected: String,
    pub loading: String,
    pub classifying: String,
    pub mono_tag: String,
}

pub(super) struct Fonts(RwLock<Catalog>);

impl Fonts {
    pub fn new(catalog: Catalog) -> Self {
        Self(RwLock::new(catalog))
    }

    pub fn replace(&self, catalog: Catalog) {
        *self.0.write().unwrap_or_else(|error| error.into_inner()) = catalog;
    }
}

/// Encode the family without losing characters or allowing a control character in picker insertions.
pub(super) fn family_key(family: &str) -> String {
    use std::fmt::Write;
    let mut key = String::from("f");
    for byte in family.as_bytes() {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

pub(super) fn decode_key(key: &str) -> Option<String> {
    if key == "inherit" {
        return Some(String::new());
    }
    let hex = key.strip_prefix('f')?.as_bytes();
    if hex.len() % 2 != 0 {
        return None;
    }
    let bytes = hex
        .chunks_exact(2)
        .map(|pair| {
            let digit = |b: u8| (b as char).to_digit(16).map(|v| v as u8);
            Some((digit(pair[0])? << 4) | digit(pair[1])?)
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

impl Provider for Fonts {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let data = self.0.read().unwrap_or_else(|error| error.into_inner()).clone();
        let choices = grouped_font_choices(
            &data.installed,
            &data.bundled,
            &data.custom,
            data.monospace.as_deref(),
            &query.text,
            data.slot.prefers_monospace() && !data.all,
        );
        let count = choices.len();
        let mut answer = Answer::default();
        if let Some(label) = data
            .slot
            .inherit_label()
            .filter(|label| label.to_lowercase().contains(query.text.trim().to_lowercase().as_str()))
        {
            let mut row = Row::new("inherit", label);
            row.icon = "textformat".into();
            row.insert = Some("inherit".into());
            row.badge = data.selected.trim().is_empty().then(|| "✓".into());
            answer.rows.push(row);
        }
        let captions = choices.iter().any(|(source, _)| *source != FontSource::Installed);
        let mut last = None;
        for (source, name) in choices {
            if cancel.is_cancelled() {
                return Answer::default();
            }
            if captions && last != Some(source) {
                answer.sections.push(source.label().to_string());
                last = Some(source);
            }
            let key = family_key(&name);
            let mut row = Row::new(&key, &name);
            row.icon = "textformat".into();
            row.insert = Some(key);
            row.section = if captions { answer.sections.len() - 1 } else { 0 };
            row.font_family = (!data.symbolic.contains(&name)).then(|| name.clone());
            row.font_embedded = data.bundled.contains(&name);
            if (!data.slot.prefers_monospace() || data.all)
                && data.monospace.as_ref().is_some_and(|mono| mono.contains(&name))
            {
                row.detail = Some(data.mono_tag.clone());
            }
            row.badge = (name == data.selected).then(|| "✓".into());
            answer.rows.push(row);
        }
        let status = if data.installed.is_empty() {
            data.loading
        } else if data.slot.prefers_monospace() && !data.all && data.monospace.is_none() {
            data.classifying
        } else {
            settings_fonts_count(whole(count))
        };
        answer.notes.push(Note::Custom(status));
        answer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_keys_preserve_names_and_reject_invalid_input() {
        for name in ["JetBrains Mono", "École", "line\nbreak", ""] {
            assert_eq!(decode_key(&family_key(name)).as_deref(), Some(name));
        }
        assert_eq!(decode_key("inherit").as_deref(), Some(""));
        assert_eq!(decode_key("f0"), None);
        assert_eq!(decode_key("fzz"), None);
    }

    #[test]
    fn font_catalog_keeps_groups_filter_and_symbol_fallback() {
        let names = ["Menlo", "Arial", "Symbols"].map(String::from);
        let data = Catalog {
            slot: FontSlot::Code,
            installed: Arc::new(names.to_vec()),
            bundled: vec!["Bundled".into()],
            custom: vec![],
            monospace: Some(Arc::new(["Menlo".into(), "Bundled".into()].into_iter().collect())),
            symbolic: Arc::new(["Symbols".into()].into_iter().collect()),
            all: false,
            selected: "Menlo".into(),
            loading: "Loading".into(),
            classifying: "Classifying".into(),
            mono_tag: "Monospace".into(),
        };
        let provider = Fonts::new(data.clone());
        let query =
            |text: &str| provider.query(&Query { text: text.into(), ..Query::default() }, &Cancel::new());
        let answer = query("");
        assert!(answer.rows.iter().any(|row| row.label == "Menlo" && row.badge.is_some()));
        assert!(answer.rows.iter().find(|row| row.label == "Bundled").unwrap().font_embedded);
        assert!(!answer.rows.iter().find(|row| row.label == "Menlo").unwrap().font_embedded);
        assert!(!answer.rows.iter().any(|row| row.label == "Arial"));
        assert!(answer.sections.iter().any(|label| label == "Bundled"));
        provider.replace(Catalog { all: true, ..data });
        let symbols = query("Symbols");
        assert_eq!(symbols.rows.len(), 1);
        assert_eq!(symbols.rows[0].font_family, None);
        assert_eq!(decode_key(symbols.rows[0].insert.as_deref().unwrap()).as_deref(), Some("Symbols"));
    }
}
