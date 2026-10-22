//! Setting the language loads it then and there, and every message reads from its table afterwards.
//!
//! Its own test binary: the language in use is process-wide, and this test is the one that changes it.

use wormhole_text::keys::*;
use wormhole_text::{Language, is_loaded, language, negotiate, set_language, text};

#[test]
fn setting_a_language_loads_it_at_once_and_only_once() {
    assert_eq!(language(), Language::ENGLISH);
    assert!(!is_loaded(Language::PSEUDO), "nothing loads a language before it is asked for");

    set_language(Language::PSEUDO);
    assert!(is_loaded(Language::PSEUDO), "loaded by the call, not by the first message");
    assert_eq!(language(), Language::PSEUDO);
    let first = text(SETTINGS_SECTION_FONTS);
    assert_eq!(first, "⟦Föñtš··⟧");

    // The same table again: the string is the one already built, at the same address.
    set_language(Language::PSEUDO);
    assert!(std::ptr::eq(first, text(SETTINGS_SECTION_FONTS)));

    set_language(Language::ENGLISH);
    assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts");
    assert!(is_loaded(Language::PSEUDO), "a language stays loaded for the next switch");
    // And back: a switch is a pointer read, the pseudo-language's table is still the same one.
    set_language(Language::PSEUDO);
    assert!(std::ptr::eq(first, text(SETTINGS_SECTION_FONTS)));
    set_language(Language::ENGLISH);
}

#[test]
fn the_system_languages_are_negotiated_to_a_shipped_one() {
    assert_eq!(negotiate(&["en-US"]), Language::ENGLISH);
    assert_eq!(
        negotiate(&["de-DE", "en-US"]),
        Language::ENGLISH,
        "no German ships yet: the next wish, then English"
    );
    assert_eq!(negotiate(&[] as &[&str]), Language::ENGLISH);
    assert_eq!(negotiate(&["not a tag", "xx"]), Language::ENGLISH);
    // The pseudo-language is asked for by name; a system never lists it.
    assert_eq!(negotiate(&["en-XA"]), Language::ENGLISH);
    assert_eq!(Language::from_tag("en-xa"), Some(Language::PSEUDO));
    assert_eq!(Language::from_tag("en"), Some(Language::ENGLISH));
    assert_eq!(Language::from_tag("tlh"), None);
}
