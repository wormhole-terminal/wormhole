//! The rules for translations (`repo-docs/docs/languages.md` §3 group 3), run on every language
//! directory under `locales/` (none yet but English) and on the fixtures in `tests/fixtures/`, which
//! show that each rule fails when it should.
//!
//! Fixtures: `en.ftl` is their English; `ok/<tag>.ftl` must pass; `bad/<name>.<tag>.ftl` must fail,
//! and its first line `# expect: <words>` is what some complaint must contain.

// The coverage line is the point of one test: it is read with `--nocapture`.
#![allow(clippy::print_stdout)]

mod support;

use std::fs;

use support::catalog::Catalog;
use support::{check, crate_dir, languages, locales_dir, read_dir};

#[test]
fn every_translation_keeps_the_rules() {
    let english = support::english();
    for language in languages().iter().filter(|l| *l != "en") {
        let translation = Catalog::parse(&read_dir(&locales_dir().join(language)));
        let problems = check::translation_problems(&english, language, &translation);
        assert!(problems.is_empty(), "locales/{language}:\n{}", problems.join("\n"));
    }
}

/// `cargo test -p wormhole-text -- coverage --nocapture`
#[test]
fn coverage_of_each_language() {
    let english = support::english();
    for language in languages().iter().filter(|l| *l != "en") {
        let translation = Catalog::parse(&read_dir(&locales_dir().join(language)));
        println!("{}", check::coverage(&english, language, &translation));
    }
    println!("{} messages in English", english.messages.len());
}

fn fixtures() -> std::path::PathBuf {
    crate_dir().join("tests/fixtures")
}

fn fixture(path: &std::path::Path) -> Catalog {
    let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Catalog::parse(&[(
        path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        source,
    )])
}

fn files_in(dir: &str) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(fixtures().join(dir))
        .unwrap_or_else(|e| panic!("fixtures/{dir}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    paths.sort();
    paths
}

#[test]
fn the_fixture_english_is_well_formed() {
    let problems = check::english_problems(&fixture(&fixtures().join("en.ftl")));
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_good_fixtures_pass() {
    let english = fixture(&fixtures().join("en.ftl"));
    let paths = files_in("ok");
    assert!(!paths.is_empty());
    for path in paths {
        let tag = path.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let problems = check::translation_problems(&english, &tag, &fixture(&path));
        assert!(problems.is_empty(), "{}:\n{}", path.display(), problems.join("\n"));
    }
}

#[test]
fn each_bad_fixture_fails_for_its_reason() {
    let english = fixture(&fixtures().join("en.ftl"));
    let paths = files_in("bad");
    assert!(paths.len() >= 12, "one fixture per rule");
    for path in paths {
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let tag =
            name.rsplit_once('.').and_then(|(stem, _)| stem.rsplit_once('.')).map_or("", |(_, tag)| tag);
        let source = fs::read_to_string(&path).expect("a fixture is text");
        let expect = source
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("# expect: "))
            .unwrap_or_else(|| panic!("{name}: the first line must be `# expect: <words>`"));
        let problems = check::translation_problems(&english, tag, &fixture(&path));
        assert!(
            problems.iter().any(|p| p.contains(expect)),
            "{name}: expected a problem containing {expect:?}, got:\n{}",
            problems.join("\n")
        );
    }
}

#[test]
fn english_that_breaks_the_rules_is_reported() {
    // A message with variables and no comment, a reference to nothing, a function call.
    let source = "plain = Fine\nopen = Open { $name } { missing } { NUMBER($n) }\n";
    let catalog = Catalog::parse(&[("bad.ftl".into(), source.into())]);
    let problems = check::english_problems(&catalog).join("\n");
    for expected in ["no comment above it", "refers to the message missing", "calls NUMBER()"] {
        assert!(problems.contains(expected), "{expected:?} not in:\n{problems}");
    }
}

#[test]
fn plural_categories_are_found_by_selecting() {
    let categories = |tag| check::plural_categories(tag).expect("the language has rules");
    assert_eq!(categories("en"), ["one", "other"].into());
    assert_eq!(categories("ja"), ["other"].into());
    assert_eq!(categories("de"), ["one", "other"].into());
    assert_eq!(categories("ru"), ["few", "many", "one", "other"].into());
    assert_eq!(categories("ar"), ["few", "many", "one", "other", "two", "zero"].into());
}
