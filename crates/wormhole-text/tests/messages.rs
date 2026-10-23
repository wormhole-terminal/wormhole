//! The English catalog through the generated keys and functions, and the pseudo-language over all of it.

mod support;

use wormhole_text::keys::*;
use wormhole_text::{Key, Language, text, with_language};

#[test]
fn messages_without_variables_read_as_written() {
    assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts");
    assert_eq!(text(SETTINGS_SECTION_SESSIONS), "Terminals & sidebar");
    assert_eq!(text(SETTINGS_APPEARANCE_THEME_MODE), "Mode");
    assert_eq!(
        text(SETTINGS_APPEARANCE_THEME_MODE_DESCRIPTION),
        "Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode"
    );
    assert_eq!(SETTINGS_APPEARANCE_THEME_MODE_KEYWORDS.id(), "settings-appearance-theme-mode.keywords");
}

#[test]
fn a_term_is_filled_in_where_a_message_uses_it() {
    assert_eq!(
        text(SETTINGS_AGENTS_OTHER_DESCRIPTION),
        "Agents Wormhole recognises that aren't on this Mac's PATH"
    );
}

#[test]
fn messages_with_variables_are_typed_functions() {
    assert_eq!(settings_open_section("Fonts"), "Open Fonts");
    assert_eq!(settings_fonts_count(1), "1 font");
    assert_eq!(settings_fonts_count(0), "0 fonts");
    assert_eq!(settings_fonts_count(5), "5 fonts");
    assert_eq!(settings_git_tab_width_value(1), "1 space");
    assert_eq!(settings_git_tab_width_value(4), "4 spaces");
}

#[test]
fn a_variable_keeps_what_it_is_given_without_isolation_marks() {
    let shown = settings_open_section("Terminals & sidebar");
    assert_eq!(shown, "Open Terminals & sidebar");
    assert!(!shown.contains(['\u{2068}', '\u{2069}']));
}

/// The keys of every message without variables, as the generator listed them.
fn plain_keys() -> Vec<Key> {
    ALL.to_vec()
}

#[test]
fn every_message_in_the_pseudo_language_is_wrapped_accented_and_longer() {
    let keys = plain_keys();
    assert!(!keys.is_empty());
    with_language(Language::PSEUDO, || {
        for key in keys {
            let english = with_language(Language::ENGLISH, || text(key));
            let pseudo = text(key);
            assert!(pseudo.starts_with('⟦') && pseudo.ends_with('⟧'), "{}: {pseudo}", key.id());
            let (a, b) = (english.chars().count(), pseudo.chars().count());
            assert!(
                b >= a + 2 + (2 * a).div_ceil(5),
                "{}: {pseudo} is not 40% longer than {english}",
                key.id()
            );
            assert_ne!(pseudo, english);
        }
    });
}

#[test]
fn a_message_with_variables_keeps_its_plural_in_the_pseudo_language() {
    with_language(Language::PSEUDO, || {
        assert_eq!(settings_fonts_count(1), "⟦1 föñt···⟧");
        assert_eq!(settings_fonts_count(5), "⟦5 föñtš···⟧");
        assert_eq!(settings_open_section("Fonts"), "⟦Öpéñ Fonts····⟧");
        for (id, text) in wormhole_text::keys::samples() {
            assert!(text.starts_with('⟦') && text.ends_with('⟧'), "{id}: {text}");
        }
    });
}

#[test]
fn the_override_is_this_threads_and_ends_with_the_closure() {
    assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts");
    with_language(Language::PSEUDO, || {
        assert_ne!(text(SETTINGS_SECTION_FONTS), "Fonts");
        std::thread::spawn(|| assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts")).join().expect("thread");
    });
    assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts");
    let panicked = std::panic::catch_unwind(|| with_language(Language::PSEUDO, || panic!("a test failed")));
    assert!(panicked.is_err());
    assert_eq!(text(SETTINGS_SECTION_FONTS), "Fonts", "a panic inside does not leave the language set");
}

#[test]
fn all_lists_every_key_once() {
    let ids: std::collections::BTreeSet<&str> = ALL.iter().map(|key| key.id()).collect();
    assert_eq!(ids.len(), ALL.len(), "a key is listed twice");
    assert!(ids.contains("settings-appearance-theme-mode.keywords"));
    assert!(ALL.iter().all(|key| !key.id().is_empty()));
    // The generator's list is the catalog's messages without variables.
    let english = support::english();
    let plain = english.messages.iter().filter(|item| english.variables(&item.id).is_empty()).count();
    assert_eq!(ALL.len(), plain);
}
