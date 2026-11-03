//! The window's words in the active language: every id of `words_table` with its text, for the window to show
//! instead of its English (which it keeps only as the source of the id). Changing the language changes the
//! snapshot's words, and the settings global already notifies the window.

use serde_json::{Map, Value};
use wormhole_text::text;

use super::words_table::WORDS;

pub(super) fn snapshot() -> Value {
    Value::Object(
        WORDS
            .iter()
            .map(|(id, key)| ((*id).to_string(), Value::String(text(*key).to_string())))
            .collect::<Map<_, _>>(),
    )
}

/// The id the window gives a word (`scripts/native-words.py`, Store.swift `wordID`): a slug of its English and the
/// FNV-1a hash of all of it.
fn id_of(english: &str) -> String {
    let hash = english.bytes().fold(0x811C_9DC5_u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    let mut slug = String::new();
    for ch in english.to_lowercase().chars().take(40) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() { format!("native-{hash:08x}") } else { format!("native-{slug}-{hash:08x}") }
}

/// One of the window's own words (those in `words.txt`) in the active language, for a row the app lists itself.
/// A word not in the table is returned as written.
pub(super) fn word(english: &str) -> String {
    let id = id_of(english);
    WORDS
        .iter()
        .find(|(known, _)| *known == id)
        .map_or_else(|| english.to_string(), |(_, key)| text(*key).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_of_the_table_is_found_by_its_english() {
        assert_eq!(word("Avatar letter"), "Avatar letter");
        assert!(
            WORDS.iter().any(|(id, _)| *id == id_of("Avatar letter")),
            "the id differs from the generator's"
        );
    }
}
