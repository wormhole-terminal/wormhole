use super::*;

#[test]
fn scrollbacks_round_trip_and_unchanged_ones_are_not_written() {
    let dir = temp_dir("scrollbacks");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b"), session(3, "/c")])).expect("saves");

    let items = [(1, "one".to_string()), (2, "two".to_string()), (3, "three".to_string())];
    assert_eq!(store.save_scrollbacks(&items).expect("saves"), 3);
    let loaded = store.load().expect("loads");
    let texts: Vec<Option<&str>> = loaded[0].sessions.iter().map(|s| s.scrollback.as_deref()).collect();
    assert_eq!(texts, [Some("one"), Some("two"), Some("three")]);

    // Same texts again: nothing is written (not even by hash collision with itself).
    let before = changes(&store);
    assert_eq!(store.save_scrollbacks(&items).expect("saves"), 0);
    assert_eq!(changes(&store), before);
    // One changed text is the only write; `&str` items work too.
    assert_eq!(store.save_scrollbacks(&[(1, "one"), (2, "two!"), (3, "three")]).expect("saves"), 1);
    assert_eq!(changes(&store), before + 1);
    assert_eq!(store.load().expect("loads")[0].sessions[1].scrollback.as_deref(), Some("two!"));
    // The one-terminal call shares the logic.
    store.save_scrollback(2, "two!").expect("saves");
    assert_eq!(changes(&store), before + 1);
    assert_eq!(store.save_scrollbacks::<&str>(&[]).expect("saves"), 0);
}

#[test]
fn scrollbacks_of_unknown_terminals_are_skipped_and_a_batch_is_all_or_nothing() {
    let dir = temp_dir("scrollbacks-atomic");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b"), session(3, "/c")])).expect("saves");
    // A terminal that has no row yet (opened since the last window save) is skipped, never an orphan.
    assert_eq!(store.save_scrollbacks(&[(99, "early")]).expect("saves"), 0);
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback"), 0);

    // One transaction: when the third write fails, the first two are not kept.
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER fail_third BEFORE INSERT ON scrollback WHEN NEW.tab_id = 3
             BEGIN SELECT RAISE(ABORT, 'disk full'); END;",
        )
        .expect("trigger");
    let items = [(1, "a".to_string()), (2, "b".to_string()), (3, "c".to_string())];
    assert!(store.save_scrollbacks(&items).is_err());
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback"), 0);
    store.conn.execute_batch("DROP TRIGGER fail_third").expect("drop");
    assert_eq!(store.save_scrollbacks(&items).expect("saves"), 3);
}

#[test]
fn closing_sessions_and_windows_forgets_their_scrollback() {
    let dir = temp_dir("scrollback-cascade");
    let mut store = Store::open_at(&dir).expect("opens");
    store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b"), session(3, "/c")])).expect("saves");
    store.save_window(&window(2, vec![session(4, "/d")])).expect("saves");
    let all: Vec<(i64, String)> = (1..=4).map(|id| (id, format!("text {id}"))).collect();
    store.save_scrollbacks(&all).expect("saves");

    // Sessions 2 and 3 closed.
    store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback WHERE tab_id IN (2, 3)"), 0);
    store.delete_window(2).expect("deletes");
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback"), 1, "only session 1's is left");
    assert_eq!(count(&store, "SELECT tab_id FROM scrollback"), 1);
}

#[test]
fn saving_a_window_never_writes_scrollback_pages() {
    // What was slow: a terminal's scrollback sat in its `tabs` row, so a window save that changed a
    // cwd had to rewrite the row, overflow chain included. Counted as WAL growth.
    let dir = temp_dir("wal-growth");
    let mut store = Store::open_at(&dir).expect("opens");
    let mut record = window(1, (1..=20).map(|id| session(id, "/work/p")).collect());
    store.save_window(&record).expect("saves");
    let text = "a line of terminal output\n".repeat(12_000); // 300 KB
    let items: Vec<(i64, String)> = (1..=20).map(|id| (id, text.clone())).collect();
    store.save_scrollbacks(&items).expect("saves");
    store.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).expect("checkpoint");

    for session in &mut record.sessions {
        session.cwd = Some("/work/a/much/longer/directory/than/before".into());
        session.notes.push(note("typed"));
    }
    store.save_window(&record).expect("saves");
    let wal = std::fs::metadata(dir.join("wormhole.db-wal")).expect("wal").len();
    assert!(wal < 128 * 1024, "a window save wrote {wal} bytes of WAL, with 6 MB of scrollback stored");
}

/// What saving costs with a realistic window (20 sessions x 10 notes x 2 KB, 300 KB of scrollback per
/// tab). Run: `cargo test -p wormhole-core store_save_costs -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn store_save_costs() {
    use std::time::Instant;
    let dir = temp_dir("bench");
    let mut store = Store::open_at(&dir).expect("opens");
    let body = "note line\n".repeat(200);
    let mut record = window(1, Vec::new());
    for id in 1..=20 {
        let mut c = session(id, "/work/project");
        c.notes = (0..10).map(|_| NoteRecord { body: body.clone(), collapsed: false }).collect();
        record.sessions.push(c);
    }
    let scrollback: Vec<(i64, String)> = (0..20)
        .map(|tab| {
            let text =
                (0..8000).map(|line| format!("tab {tab} line {line} output of a build step\n")).collect();
            (tab + 1, text)
        })
        .collect();
    let kb = scrollback[0].1.len() / 1000;
    store.save_window(&record).expect("saves");
    let t = Instant::now();
    for (id, text) in &scrollback {
        store.save_scrollback(*id, text).expect("scrollback");
    }
    eprintln!("20 x save_scrollback, {kb} KB each, first pass: {:?}", t.elapsed());
    let t = Instant::now();
    for (id, text) in &scrollback {
        store.save_scrollback(*id, text).expect("scrollback");
    }
    eprintln!("20 x save_scrollback, unchanged: {:?}", t.elapsed());
    // The same 20 changed texts, one transaction per tab and in one transaction.
    let edited = |suffix: &str| -> Vec<(i64, String)> {
        scrollback.iter().map(|(id, t)| (*id, format!("{t}{suffix}\n"))).collect()
    };
    let (one, batch) = (edited("one by one"), edited("batched"));
    let t = Instant::now();
    for (id, text) in &one {
        store.save_scrollback(*id, text).expect("scrollback");
    }
    eprintln!("20 x save_scrollback, all changed: {:?}", t.elapsed());
    let t = Instant::now();
    store.save_scrollbacks(&batch).expect("scrollback");
    eprintln!("save_scrollbacks, all 20 changed: {:?}", t.elapsed());
    let t = Instant::now();
    store.save_scrollbacks(&batch).expect("scrollback");
    eprintln!("save_scrollbacks, none changed (20 x 310 KB hashed): {:?}", t.elapsed());
    // Same again with an fsync per commit, as the database was opened before.
    store.conn.pragma_update(None, "synchronous", "FULL").expect("pragma");
    let (one, batch) = (edited("one by one, full"), edited("batched, full"));
    let t = Instant::now();
    for (id, text) in &one {
        store.save_scrollback(*id, text).expect("scrollback");
    }
    eprintln!("synchronous=FULL: 20 x save_scrollback: {:?}", t.elapsed());
    let t = Instant::now();
    store.save_scrollbacks(&batch).expect("scrollback");
    eprintln!("synchronous=FULL: save_scrollbacks: {:?}", t.elapsed());
    store.conn.pragma_update(None, "synchronous", "NORMAL").expect("pragma");
    // A checkpoint lands on whichever commit crosses 1000 WAL pages; settle it first.
    store.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).expect("checkpoint");
    let t = Instant::now();
    for _ in 0..20 {
        store.save_window(&record).expect("saves");
    }
    eprintln!("save_window, unchanged, scrollback present: {:?} per save", t.elapsed() / 20);
    let t = Instant::now();
    for i in 0..20 {
        record.sessions[3].notes[2].body.push_str(&format!(" typed {i}"));
        store.save_window(&record).expect("saves");
    }
    eprintln!("save_window after one note edit: {:?} per save", t.elapsed() / 20);
    let t = Instant::now();
    for i in 0..20 {
        record.sessions[3].notes[2].body.push_str(&format!(" more {i}"));
        store.save_session(1, &record.sessions[3]).expect("saves");
    }
    eprintln!("save_session after one note edit: {:?} per save", t.elapsed() / 20);
    let t = Instant::now();
    let loaded = store.load().expect("loads");
    eprintln!("load: {:?} ({} sessions)", t.elapsed(), loaded[0].sessions.len());
}
