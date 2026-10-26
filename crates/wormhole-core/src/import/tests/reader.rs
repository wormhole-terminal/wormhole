//! The reader's limits: size cap, file count, depth, cycles; and that links are the person's own.

use super::Fixture;
use crate::import::read::{MAX_FILES, Reader};
use crate::import::{Reason, Setting, Source};

#[test]
fn a_file_over_the_cap_is_unreadable_not_truncated() {
    let fixture = Fixture::new();
    let big = "# padding\n".repeat(120_000);
    assert!(big.len() > 1024 * 1024);
    fixture.home(".config/ghostty/config", &format!("{big}font-size = 20\n"));
    let found = fixture.found(Source::Ghostty);
    assert!(found.proposal.values.is_empty());
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::Unreadable));
}

#[test]
fn a_file_that_is_not_utf8_is_unreadable() {
    let fixture = Fixture::new();
    fixture.bytes(&fixture.env.home.join(".config/ghostty/config"), &[0xff, 0xfe, 0x00, b'a']);
    assert!(fixture.found(Source::Ghostty).proposal.skipped.iter().any(|s| s.reason == Reason::Unreadable));
}

#[test]
fn a_directory_where_a_file_should_be_is_unreadable_and_a_missing_file_is_quiet() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.env.home.join(".config/ghostty/config")).expect("dir");
    // `locations` only lists files, so a folder of that name means there is no Ghostty config.
    assert!(crate::import::scan_one(&fixture.env, Source::Ghostty).is_none());

    let mut reader = Reader::new(fixture.root(), 4);
    assert!(reader.text(&fixture.root().join("nowhere")).is_none());
    let (files, skipped) = reader.finish();
    assert!(files.is_empty() && skipped.is_empty());
}

#[test]
fn at_most_a_fixed_number_of_files_are_read() {
    let fixture = Fixture::new();
    let mut config = String::new();
    for n in 0..(MAX_FILES + 8) {
        config.push_str(&format!("config-file = inc{n}.conf\n"));
        fixture.home(&format!(".config/ghostty/inc{n}.conf"), &format!("font-size = {}\n", 10 + n % 5));
    }
    fixture.home(".config/ghostty/config", &config);
    let found = fixture.found(Source::Ghostty);
    assert_eq!(found.files.len(), MAX_FILES);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeLimit));
}

#[test]
fn a_chain_of_includes_stops_at_the_depth_limit() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "config-file = d1.conf\n");
    for n in 1..12 {
        fixture.home(
            &format!(".config/ghostty/d{n}.conf"),
            &format!("config-file = d{}.conf\nbackground-opacity = 0.{}\n", n + 1, n % 10),
        );
    }
    let found = fixture.found(Source::Ghostty);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeLimit));
    assert!(found.proposal.get(Setting::Opacity).is_some());
}

#[test]
fn an_include_that_is_a_link_is_followed_as_the_persons_own_file_and_a_link_loop_is_a_cycle() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "config-file = link.conf\n");
    let target = fixture.home("dotfiles/ghostty.conf", "font-size = 17\nconfig-file = link.conf\n");
    crate::platform::fs::symlink(&target, fixture.env.home.join(".config/ghostty/link.conf"))
        .expect("symlink");
    let found = fixture.found(Source::Ghostty);
    assert_eq!(super::number(&found.proposal, Setting::FontSize), 17.0);
    // The file includes the link that is itself: one real file, read once.
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeCycle));
}
