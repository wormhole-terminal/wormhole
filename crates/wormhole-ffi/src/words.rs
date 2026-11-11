//! The interface's words (`wormhole-text`) for Swift: one fetch of a language's whole table, then an index, never
//! a call per word. A message's place in the table is its place in the id list, which is also a `Key`'s index in
//! Rust, so a table and the id list always agree; the Swift side checks `text_hash()` against the one its
//! generated keys were made from.

use wormhole_text::{Language, catalog, negotiate, set_language};

/// A language the interface can be shown in.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LanguageRecord {
    /// The BCP 47 tag: `en`, `de`, `en-XA`.
    pub tag: String,
    /// The pseudo-language, for testing only: never offered to a person.
    pub pseudo: bool,
}

/// Every language this build can show, English first and the pseudo-language last.
#[uniffi::export]
pub fn text_languages() -> Vec<LanguageRecord> {
    Language::all().map(|l| LanguageRecord { tag: l.tag().to_string(), pseudo: l.is_pseudo() }).collect()
}

/// The shipped language that best matches the person's preferred ones (most wanted first), English when none does.
#[uniffi::export]
pub fn text_negotiate(requested: Vec<String>) -> String {
    negotiate(&requested).tag().to_string()
}

/// Every message id (`settings-theme-mode`, `settings-theme-mode.description`), in the order of the tables.
#[uniffi::export]
pub fn text_ids() -> Vec<String> {
    catalog(Language::ENGLISH).map(|(id, _)| id.to_string()).collect()
}

/// A language's table, the same length as `text_ids()`: the text of each message, English's where the language
/// lacks it, and "" for a message with variables (those are formatted by Rust, by their typed function). Nil for
/// a tag no language has.
#[uniffi::export]
pub fn text_table(tag: String) -> Option<Vec<String>> {
    let language = Language::from_tag(&tag)?;
    Some(catalog(language).map(|(_, text)| text.to_string()).collect())
}

/// Makes `tag` the language of the words Rust itself writes (a banner's title, a menu's); false for an
/// unknown tag. The tables Swift fetched do not change: it fetches again.
#[uniffi::export]
pub fn text_set_language(tag: String) -> bool {
    Language::from_tag(&tag).map(set_language).is_some()
}

/// A variable of a message formatted by `text_format`: its name and a value, a whole number when `whole` is set (a
/// plural selects on it) and `text` otherwise.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextArg {
    pub name: String,
    pub text: String,
    pub whole: Option<i64>,
}

/// The message called `id` with `args` in the language in use, for a message with variables (the table holds none
/// of those); nil for an id no message has.
#[uniffi::export]
pub fn text_format(id: String, args: Vec<TextArg>) -> Option<String> {
    let values: Vec<(&str, wormhole_text::Value<'_>)> = args
        .iter()
        .map(|a| (a.name.as_str(), a.whole.map_or(wormhole_text::Value::Text(&a.text), wormhole_text::Value::Whole)))
        .collect();
    wormhole_text::format(&id, &values)
}

/// A hash of the id list, so a front end whose keys were generated from another list knows it is stale (FNV-1a,
/// which is stable across builds, unlike `std`'s hasher).
#[uniffi::export]
pub fn text_hash() -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (id, _) in catalog(Language::ENGLISH) {
        for byte in id.bytes().chain(std::iter::once(b'\n')) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_as_long_as_the_ids_and_english_has_every_plain_message() {
        let ids = text_ids();
        let table = text_table("en".into()).expect("English");
        assert_eq!(ids.len(), table.len());
        assert!(ids.len() > 1500, "{}", ids.len());
        let at = ids.iter().position(|id| id == "folder-picker-empty").expect("a message");
        assert_eq!(table[at], "No folders");
        // A message with variables is empty in the table.
        let cut =
            ids.iter().position(|id| id == "folder-picker-cut-folders").expect("a message with a variable");
        assert_eq!(table[cut], "");
    }

    #[test]
    fn the_pseudo_language_changes_every_plain_word_and_an_unknown_tag_is_none() {
        let english = text_table("en".into()).expect("en");
        let pseudo = text_table("en-XA".into()).expect("en-XA");
        let at = text_ids().iter().position(|id| id == "folder-picker-empty").expect("a message");
        assert_ne!(pseudo[at], english[at]);
        assert!(pseudo[at].contains('⟦'), "{}", pseudo[at]);
        assert!(text_table("xx".into()).is_none());
        assert!(!text_set_language("xx".into()));
        assert_eq!(text_negotiate(vec!["de-DE".into(), "en-US".into()]), "en", "only English ships");
        let languages = text_languages();
        assert_eq!(languages.first().map(|l| l.tag.as_str()), Some("en"));
        assert!(languages.last().is_some_and(|l| l.pseudo));
    }

    #[test]
    fn the_hash_is_stable_and_tracks_the_id_list() {
        assert_eq!(text_hash(), text_hash());
        assert_ne!(text_hash(), 0);
    }
}
