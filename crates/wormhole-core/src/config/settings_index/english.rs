//! The migration guard: every word of the table, in English, is what it was before the words moved to
//! `wormhole-text`'s catalogs. `english.txt` is the table as it was written in Rust, and this test
//! reads it back through the keys, byte for byte. A deliberate change to the English wording is made in
//! `locales/en/` and here. The same table in the pseudo-language shows that none of it is a literal.

use std::fmt::Write as _;

use super::{SectionId, entries};

const GOLDEN: &str = include_str!("english.txt");

/// The table as text: each section's page words, each group's, each setting's.
fn dump() -> String {
    let mut out = String::new();
    for section in SectionId::ALL {
        let info = section.info();
        let _ = writeln!(out, "section {section:?}\n  title {:?}\n  intro {:?}", info.title(), info.intro());
        let _ = writeln!(out, "  nav {:?}", info.nav.title());
        for group in section.groups() {
            let _ = writeln!(
                out,
                "group {:?}\n  blurb {:?}\n  aside {:?}",
                group.title(),
                group.blurb(),
                group.aside()
            );
        }
    }
    for (section, _, entry) in entries() {
        let _ = writeln!(
            out,
            "entry {section:?} {}\n  label {:?}\n  description {:?}\n  keywords {:?}",
            entry.id,
            entry.label(),
            entry.description(),
            entry.keywords()
        );
    }
    out
}

#[test]
fn the_table_says_in_english_what_it_always_said() {
    let now = dump();
    if std::env::var_os("SETTINGS_ENGLISH_BLESS").is_some() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/settings_index/english.txt");
        std::fs::write(path, &now).expect("writes the golden file");
        return;
    }
    for (at, (new, old)) in now.lines().zip(GOLDEN.lines()).enumerate() {
        assert_eq!(new, old, "line {} of english.txt", at + 1);
    }
    assert_eq!(now.lines().count(), GOLDEN.lines().count(), "the table and english.txt differ in length");
}

/// In `en-XA` every message is accented and in `⟦ ⟧`, so a word of the table still written in Rust shows.
#[test]
fn no_word_of_the_table_is_left_in_rust() {
    use wormhole_text::{Language, with_language};
    let said = |text: &str| text.is_empty() || (text.starts_with('⟦') && text.ends_with('⟧'));
    with_language(Language::PSEUDO, || {
        for section in SectionId::ALL {
            let info = section.info();
            assert!(said(info.title()) && said(info.intro()), "{section:?}");
            for group in section.groups() {
                assert!(said(group.title()) && said(group.blurb()) && said(group.aside()), "{section:?}");
            }
        }
        for (_, _, entry) in entries() {
            let words = [entry.label(), entry.description(), entry.keywords()];
            assert!(words.into_iter().all(said), "{} has a word that is not a message", entry.id);
            assert!(!entry.label().is_empty(), "{}", entry.id);
        }
    });
}
