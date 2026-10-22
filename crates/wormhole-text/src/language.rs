//! The languages: the directories under `locales/`, English first, and the pseudo-language last.

use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use unic_langid::LanguageIdentifier;

use crate::generated::files::{LOCALES, Locale};

/// The pseudo-language's tag: CLDR's accented English, for finding hard-coded and truncated text.
pub(crate) const PSEUDO_TAG: &str = "en-XA";

/// A language the interface can be shown in. Cheap to copy; the names come from the catalogs, so a
/// language exists exactly when `locales/<tag>/` does (plus the pseudo-language).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Language(u8);

impl Language {
    /// The source language, always there and always the fallback.
    pub const ENGLISH: Language = Language(0);
    /// English made unreadable on purpose (accented, 40% longer, in `⟦ ⟧`). Never negotiated
    /// from the system's list; devctl and the tests ask for it by name.
    pub const PSEUDO: Language = Language(LOCALES.len() as u8);
    /// The shipped languages and the pseudo-language.
    pub(crate) const COUNT: usize = LOCALES.len() + 1;

    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: u8) -> Self {
        Language(index.min(Self::PSEUDO.0))
    }

    /// Whether this is `en-XA`, the pseudo-language: not a language to offer a person, only to test with.
    pub const fn is_pseudo(self) -> bool {
        self.0 == Self::PSEUDO.0
    }

    /// The BCP 47 tag: `en`, `de`, `en-XA`.
    pub fn tag(self) -> &'static str {
        self.locale().map_or(PSEUDO_TAG, |l| l.tag)
    }

    /// The language with this tag, ignoring case; the pseudo-language included.
    pub fn from_tag(tag: &str) -> Option<Language> {
        Self::all().find(|l| l.tag().eq_ignore_ascii_case(tag))
    }

    /// The languages with real text, English first.
    pub fn shipped() -> impl Iterator<Item = Language> {
        (0..LOCALES.len() as u8).map(Language)
    }

    /// Every language this build can show, the pseudo-language last.
    pub fn all() -> impl Iterator<Item = Language> {
        (0..Self::COUNT as u8).map(Language)
    }

    /// The catalog files; `None` for the pseudo-language, which is made from English.
    pub(crate) fn locale(self) -> Option<&'static Locale> {
        LOCALES.get(self.index())
    }
}

/// The shipped language that best matches the user's preferred languages (most wanted first, as
/// macOS lists them: `["de-DE", "en-US"]`), by Fluent's lookup strategy: the first request one
/// language answers, the language itself or a parent of it (`de-AT` gets `de`). English when none
/// does. Tags that are not valid are skipped.
pub fn negotiate(requested: &[impl AsRef<str>]) -> Language {
    let requested: Vec<LanguageIdentifier> =
        requested.iter().filter_map(|t| t.as_ref().parse().ok()).collect();
    let available: Vec<LanguageIdentifier> =
        Language::shipped().filter_map(|l| l.tag().parse().ok()).collect();
    let default: LanguageIdentifier = Language::ENGLISH.tag().parse().unwrap_or_default();
    negotiate_languages(&requested, &available, Some(&default), NegotiationStrategy::Lookup)
        .first()
        .and_then(|best| Language::shipped().find(|l| l.tag() == best.to_string()))
        .unwrap_or(Language::ENGLISH)
}
