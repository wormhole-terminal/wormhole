use super::*;
use crate::storage::store::ClaimRecord;

fn claim(session_id: i64, root: &str, path: &str) -> ClaimRecord {
    ClaimRecord { session_id, root: root.into(), path: path.into() }
}

#[test]
fn claims_come_back_per_session_and_a_save_replaces_only_the_sessions_it_names() {
    let dir = temp_dir("claims-round-trip");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    store
        .save_claims(
            &[1, 2],
            &[claim(1, "/repo", "a.rs"), claim(1, "/repo", "b.rs"), claim(2, "/other", "c.rs")],
        )
        .expect("saves");
    assert_eq!(store.claims(&[1]).expect("reads"), [claim(1, "/repo", "a.rs"), claim(1, "/repo", "b.rs")]);
    assert_eq!(store.claims(&[2, 3]).expect("reads"), [claim(2, "/other", "c.rs")]);

    // Terminal 1 had `a.rs` committed; terminal 2 is not part of this save.
    store.save_claims(&[1], &[claim(1, "/repo", "b.rs")]).expect("saves");
    assert_eq!(store.claims(&[1]).expect("reads"), [claim(1, "/repo", "b.rs")]);
    assert_eq!(store.claims(&[2]).expect("reads"), [claim(2, "/other", "c.rs")]);

    // It is on disk: a store opened again has it.
    drop(store);
    let store = Store::open_at(&dir).expect("opens again");
    assert_eq!(store.claims(&[1, 2]).expect("reads").len(), 2);
}

#[test]
fn a_session_deleted_takes_its_claims_with_it_and_clearing_takes_every_ones() {
    let dir = temp_dir("claims-forgotten");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
    store.save_claims(&[1, 2], &[claim(1, "/repo", "a.rs"), claim(2, "/repo", "b.rs")]).expect("saves");
    // The window is saved again without terminal 1: it is closed for good.
    store.save_window(&window(1, vec![session(2, "/b")])).expect("saves");
    assert_eq!(store.claims(&[1]).expect("reads"), []);
    assert_eq!(store.claims(&[2]).expect("reads").len(), 1);

    store.clear_claims().expect("clears");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM row_claims"), 0);
}

#[test]
fn the_claims_of_a_terminal_that_is_gone_are_dropped_when_the_database_opens() {
    let dir = temp_dir("claims-orphans");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    // Terminal 7 was never saved (or its rows went in a crash between the two writes).
    store.save_claims(&[1, 7], &[claim(1, "/repo", "a.rs"), claim(7, "/repo", "b.rs")]).expect("saves");
    drop(store);
    let store = Store::open_at(&dir).expect("opens again");
    assert_eq!(store.claims(&[1, 7]).expect("reads"), [claim(1, "/repo", "a.rs")]);
}
