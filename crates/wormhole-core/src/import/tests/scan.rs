//! Scanning every source: who is found, in which order, and that nothing is read but the fixture.

use std::time::{Duration, SystemTime};

use super::Fixture;
use crate::import::{Env, Reason, Source, scan};

fn set_modified(path: &std::path::Path, seconds_ago: u64) {
    let when = SystemTime::now() - Duration::from_secs(seconds_ago);
    let file = std::fs::File::options().write(true).open(path).expect("open");
    file.set_modified(when).expect("set mtime");
}

#[test]
fn a_home_with_nothing_finds_nothing() {
    assert!(scan(&Fixture::new().env).is_empty());
}

#[test]
fn every_source_with_a_file_is_found_and_the_newest_comes_first() {
    let fixture = Fixture::new();
    let ghostty = fixture.home(".config/ghostty/config", "font-size = 12\n");
    let kitty = fixture.home(".config/kitty/kitty.conf", "font_size 12\n");
    let zed = fixture.home(".config/zed/settings.json", "{}");
    let alacritty = fixture.home(".config/alacritty/alacritty.toml", "[font]\nsize = 12\n");
    set_modified(&ghostty, 5000);
    set_modified(&kitty, 10);
    set_modified(&zed, 300);
    set_modified(&alacritty, 90_000);

    let found = scan(&fixture.env);
    let order: Vec<Source> = found.iter().map(|f| f.source).collect();
    assert_eq!(order, [Source::Kitty, Source::Zed, Source::Ghostty, Source::Alacritty]);
    assert!(found.iter().all(|f| f.modified.is_some()));
    assert_eq!(found[0].files, [kitty]);
}

#[test]
fn a_file_that_does_not_parse_never_fails_the_scan() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", "= = =");
    fixture.home(".config/zed/settings.json", "{ \"terminal\": ");
    fixture.home("Library/Preferences/com.apple.Terminal.plist", "nope");
    fixture.home(".warp/settings.toml", "[[[");
    fixture.home(".config/ghostty/config", "font-size = 12\n");
    let found = scan(&fixture.env);
    assert_eq!(found.len(), 5);
    for f in &found {
        if f.source != Source::Ghostty {
            assert!(f.proposal.skipped.iter().any(|s| s.reason == Reason::Unreadable), "{}", f.source.key());
        }
    }
}

#[test]
fn the_environment_decides_where_to_look_and_the_home_is_never_the_real_one() {
    // A second fixture has its own home: what the first one has is not in it.
    let one = Fixture::new();
    one.home(".config/ghostty/config", "font-size = 12\n");
    let two = Fixture::new();
    assert_eq!(scan(&one.env).len(), 1);
    assert!(scan(&two.env).is_empty());
    // An empty `Env` finds nothing anywhere: no folder is read relative to the working directory.
    assert!(scan(&Env::new(two.root().join("none"))).is_empty());
}

#[test]
fn source_keys_are_stable_and_round_trip() {
    let keys: Vec<&str> = Source::ALL.iter().map(|s| s.key()).collect();
    assert_eq!(
        keys,
        ["ghostty", "iterm2", "terminal-app", "alacritty", "kitty", "wezterm", "warp", "zed", "vscode"]
    );
    for source in Source::ALL {
        assert_eq!(Source::from_key(source.key()), Some(source));
    }
    assert_eq!(Source::from_key("nope"), None);
}
