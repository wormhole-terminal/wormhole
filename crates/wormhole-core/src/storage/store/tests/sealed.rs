//! The saved text is sealed with the profile's key: unreadable in the file, unreadable with
//! another key or once changed, and not kept at all without a key.

use std::sync::atomic::Ordering;

use super::erasure::on_disk;
use super::saved_text::FakePlatform;
use super::*;

const MARKER: &str = "SECRET-MARKER-5";

fn text() -> String {
    format!("$ gh auth token\n{MARKER}-ghp_0123456789\n").repeat(200)
}

fn store_with_text(dir: &std::path::Path, platform: std::sync::Arc<FakePlatform>) -> Store {
    let mut store = Store::open_in(dir, platform).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    assert_eq!(store.save_scrollbacks(&[(1, text()), (2, "short".into())]).expect("saves"), 2);
    store
}

#[test]
fn the_text_round_trips_and_is_never_in_the_files_in_the_clear() {
    let dir = temp_dir("sealed-round-trip");
    let store = store_with_text(&dir, FakePlatform::with_key(1));
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new(), "before a checkpoint");
    store.checkpoint().expect("checkpoint");
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new(), "after");
    drop(store);
    let store = Store::open_in(&dir, FakePlatform::with_key(1)).expect("opens");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions[0].scrollback.as_deref(), Some(text().as_str()));
    assert_eq!(loaded[0].sessions[1].scrollback.as_deref(), Some("short"));
    // What was read counts as written: the same text again writes nothing.
    let before = changes(&store);
    store.save_scrollback(1, &text()).expect("saves");
    assert_eq!(changes(&store), before);
}

#[test]
fn with_another_key_nothing_comes_back_and_the_next_save_writes_it_again() {
    let dir = temp_dir("sealed-other-key");
    drop(store_with_text(&dir, FakePlatform::with_key(1)));
    let mut store = Store::open_in(&dir, FakePlatform::with_key(2)).expect("opens");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions[0].scrollback, None);
    assert_eq!(loaded[0].sessions[0].cwd.as_deref(), Some("/a"), "the terminal itself is kept");
    assert_eq!(store.save_scrollbacks(&[(1, "new")]).expect("saves"), 1);
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback.as_deref(), Some("new"));
}

#[test]
fn a_changed_or_swapped_record_does_not_open() {
    let dir = temp_dir("sealed-tampered");
    let store = store_with_text(&dir, FakePlatform::with_key(1));
    // One bit of terminal 1's record flipped, and terminal 2's record put under terminal 1's id.
    let mut record = super::erasure::sealed_record(&store, 1);
    let last = record.len() - 1;
    record[last] ^= 0x01;
    store
        .conn
        .execute("UPDATE saved.scrollback SET sealed = ?1 WHERE tab_id = 1", [&record])
        .expect("changes");
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback, None, "changed");
    store
        .conn
        .execute_batch(
            "UPDATE saved.scrollback SET sealed = (SELECT sealed FROM saved.scrollback WHERE tab_id = 2)
             WHERE tab_id = 1;",
        )
        .expect("swaps");
    let loaded = Store::open_in(&dir, FakePlatform::with_key(1)).expect("opens").load().expect("loads");
    assert_eq!(loaded[0].sessions[0].scrollback, None, "another terminal's record");
    assert_eq!(loaded[0].sessions[1].scrollback.as_deref(), Some("short"));
}

#[test]
fn without_a_key_nothing_is_kept_and_nothing_is_written_in_the_clear() {
    let dir = temp_dir("sealed-no-key");
    let platform = FakePlatform::without_key();
    let mut store = Store::open_in(&dir, platform.clone()).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    assert_eq!(store.save_scrollbacks(&[(1, text())]).expect("saves"), 0);
    store.save_scrollback(1, &text()).expect("no error, just not kept");
    store.checkpoint().expect("checkpoint");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM saved.scrollback"), 0);
    assert_eq!(on_disk(&dir, MARKER), Vec::<String>::new());
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback, None);
    // Asked once, not at every save: a Keychain that said no isn't asked again until next launch.
    assert_eq!(platform.key_asked.load(Ordering::Relaxed), 1);
}

#[test]
fn opening_a_store_does_not_ask_for_the_key() {
    // Other profiles' databases are opened at launch for their ids only: no Keychain for that.
    let dir = temp_dir("sealed-lazy");
    let platform = FakePlatform::with_key(1);
    let store = Store::open_in(&dir, platform.clone()).expect("opens");
    store.next_ids().expect("ids");
    assert_eq!(platform.key_asked.load(Ordering::Relaxed), 0);
}

/// A development build keeps its key in a file beside the text (never the Keychain): owner-only,
/// made once and read back.
#[test]
fn a_development_build_keeps_its_key_in_an_owner_only_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("sealed-dev-key");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
        assert_eq!(store.save_scrollbacks(&[(1, "kept")]).expect("saves"), 1);
    }
    let key = dir.join("scrollback-key.dev-only");
    let mode = std::fs::metadata(&key).expect("the key file").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    assert_eq!(std::fs::read(&key).expect("reads").len(), 32);
    let store = Store::open_at(&dir).expect("opens");
    assert_eq!(store.load().expect("loads")[0].sessions[0].scrollback.as_deref(), Some("kept"));
}

#[test]
fn erasing_takes_every_terminals_text_off_the_disk() {
    let dir = temp_dir("sealed-erase");
    let mut store = store_with_text(&dir, FakePlatform::with_key(1));
    store.checkpoint().expect("checkpoint");
    let record = super::erasure::sealed_record(&store, 1)[100..164].to_vec();
    assert!(!super::erasure::bytes_on_disk(&dir, &record).is_empty());
    assert_eq!(store.erase_saved_text().expect("erases"), 2);
    assert_eq!(super::erasure::bytes_on_disk(&dir, &record), Vec::<String>::new());
    assert!(store.load().expect("loads")[0].sessions.iter().all(|s| s.scrollback.is_none()));
    // What a terminal shows next is saved again (the setting turned back on).
    assert_eq!(store.save_scrollbacks(&[(1, "again")]).expect("saves"), 1);
}

#[test]
fn a_terminal_whose_text_is_not_kept_has_none_saved() {
    let dir = temp_dir("sealed-opt-out");
    let mut store = store_with_text(&dir, FakePlatform::with_key(1));
    let mut quiet = session(1, "/a");
    quiet.forget_text = true;
    store.save_window(&window(1, vec![quiet, session(2, "/b")])).expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM saved.scrollback WHERE tab_id = 1"), 0, "deleted");
    assert_eq!(store.save_scrollbacks(&[(1, "more"), (2, "other")]).expect("saves"), 1, "only 2's");
    let loaded = store.load().expect("loads");
    assert!(loaded[0].sessions[0].forget_text, "the choice is kept");
    assert_eq!(loaded[0].sessions[0].scrollback, None);
    assert_eq!(loaded[0].sessions[1].scrollback.as_deref(), Some("other"));
    // Kept again once the person changes their mind.
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    assert_eq!(store.save_scrollbacks(&[(1, "more")]).expect("saves"), 1);
}
