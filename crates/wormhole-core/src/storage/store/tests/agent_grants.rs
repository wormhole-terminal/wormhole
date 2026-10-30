use super::migrations::version;
use super::*;
use crate::storage::store::{AgentGrantKey, AgentGrantRow};

fn key(kind: &str, agent: &str, pin: &str) -> AgentGrantKey {
    AgentGrantKey {
        kind: kind.into(),
        agent: agent.into(),
        identifier: format!("id.{agent}"),
        team_id: "ABCDE12345".into(),
        pin: pin.into(),
    }
}

fn grant(key: AgentGrantKey, caps: u32) -> AgentGrantRow {
    AgentGrantRow {
        key,
        label: "Claude Code".into(),
        location: "/bin/claude".into(),
        caps,
        created: 100,
        ..Default::default()
    }
}

#[test]
fn an_approval_is_found_by_its_key_and_renewed_not_duplicated() {
    let mut store = Store::open_at(&temp_dir("grants-key")).expect("opens");
    let id = store.save_agent_grant(&grant(key("developer_id", "claude", ""), 0b0111)).expect("saves");
    assert_eq!(store.agent_grant(&key("developer_id", "claude", "")).expect("reads").map(|g| g.id), Some(id));
    // Another pin is another program: ad-hoc programs are told apart by their hash.
    assert!(store.agent_grant(&key("developer_id", "claude", "other")).expect("reads").is_none());
    assert!(store.agent_grant(&key("developer_id", "codex", "")).expect("reads").is_none());

    // The same program approved again keeps its row and history and takes the new words and powers.
    store.touch_agent_grant(id, 150, "/bin/claude", "aa", 3).expect("touches");
    let mut again = grant(key("developer_id", "claude", ""), 0b1111);
    again.label = "Claude".into();
    assert_eq!(store.save_agent_grant(&again).expect("saves"), id);
    let all = store.agent_grants().expect("lists");
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].caps, all[0].label.as_str()), (0b1111, "Claude"));
    assert_eq!((all[0].uses, all[0].last_used, all[0].created), (3, Some(150), 100));
}

#[test]
fn revoking_removes_the_row_and_the_list_is_the_most_recently_used_first() {
    let mut store = Store::open_at(&temp_dir("grants-revoke")).expect("opens");
    let a = store.save_agent_grant(&grant(key("ad_hoc", "aider", "h1"), 1)).expect("saves");
    let b = store.save_agent_grant(&grant(key("script", "pi", "/p"), 1)).expect("saves");
    store.touch_agent_grant(a, 500, "", "", 1).expect("touches");
    let order: Vec<i64> = store.agent_grants().expect("lists").iter().map(|g| g.id).collect();
    assert_eq!(order, [a, b]);
    store.set_agent_grant_caps(b, 3).expect("sets");
    assert_eq!(store.agent_grants().expect("lists")[1].caps, 3);
    assert!(store.delete_agent_grant(a).expect("deletes"));
    assert!(!store.delete_agent_grant(a).expect("deletes again"));
    assert_eq!(store.agent_grants().expect("lists").len(), 1);
}

#[test]
fn a_database_from_version_10_gains_the_table_and_keeps_its_terminals() {
    let dir = temp_dir("grants-v10");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a")])).expect("saves");
        // Version 10 as it shipped: the same layout without the table, and without the column of version 12.
        store
            .conn
            .execute_batch("DROP TABLE agent_grants; ALTER TABLE sessions DROP COLUMN agent_hidden; ALTER TABLE sessions DROP COLUMN done_unseen;")
            .expect("drops");
        store.conn.pragma_update(None, "user_version", 10).expect("version");
        assert_eq!(version(&store.conn), 10);
    }
    let mut store = Store::open_at(&dir).expect("upgrades");
    assert!(store.recovered_from().is_none());
    assert_eq!(version(&store.conn), 13);
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions[0].cwd.as_deref(), Some("/a"));
    assert!(store.agent_grants().expect("lists").is_empty());
    store.save_agent_grant(&grant(key("scripts", "", ""), 1)).expect("the table works");
}

#[test]
fn a_terminals_hidden_flag_is_saved_and_loaded_and_the_default_is_visible() {
    let mut store = Store::open_at(&temp_dir("hidden-flag")).expect("opens");
    let mut hidden = session(2, "/b");
    hidden.agent_hidden = true;
    store.save_window(&window(1, vec![session(1, "/a"), hidden])).expect("saves");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions.iter().map(|s| s.agent_hidden).collect::<Vec<_>>(), [false, true]);
    // Switching it back on the same saved record is a write like any other change.
    let mut again = loaded[0].clone();
    again.sessions[1].agent_hidden = false;
    store.save_window(&again).expect("saves again");
    assert!(!store.load().expect("loads")[0].sessions[1].agent_hidden);
}

#[test]
fn a_database_from_version_11_gains_the_column_and_every_terminal_stays_visible() {
    let dir = temp_dir("hidden-v11");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
        // Version 11 as it shipped: the same layout without the column.
        store
            .conn
            .execute_batch("ALTER TABLE sessions DROP COLUMN agent_hidden; ALTER TABLE sessions DROP COLUMN done_unseen;")
            .expect("drops");
        store.conn.pragma_update(None, "user_version", 11).expect("version");
        assert_eq!(version(&store.conn), 11);
    }
    let mut store = Store::open_at(&dir).expect("upgrades");
    assert!(store.recovered_from().is_none(), "upgraded, not moved aside");
    assert_eq!(version(&store.conn), 13);
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions.len(), 2);
    assert!(windows[0].sessions.iter().all(|s| !s.agent_hidden));
    let mut record = windows[0].clone();
    record.sessions[0].agent_hidden = true;
    store.save_window(&record).expect("the column works");
    assert!(store.load().expect("loads")[0].sessions[0].agent_hidden);
}

#[test]
fn a_terminals_unseen_done_is_saved_loaded_and_cleared() {
    let mut store = Store::open_at(&temp_dir("done-unseen")).expect("opens");
    let mut done = session(2, "/b");
    done.done_unseen = true;
    store.save_window(&window(1, vec![session(1, "/a"), done])).expect("saves");
    let loaded = store.load().expect("loads");
    assert_eq!(loaded[0].sessions.iter().map(|s| s.done_unseen).collect::<Vec<_>>(), [false, true]);
    // Seen: the same record without it is a write like any other change.
    let mut seen = loaded[0].clone();
    seen.sessions[1].done_unseen = false;
    store.save_window(&seen).expect("saves again");
    assert!(!store.load().expect("loads")[0].sessions[1].done_unseen);
}

#[test]
fn a_database_from_version_12_gains_the_column_and_no_terminal_is_unseen() {
    let dir = temp_dir("done-unseen-v12");
    {
        let mut store = Store::open_at(&dir).expect("opens");
        store.save_window(&window(1, vec![session(1, "/a"), session(2, "/b")])).expect("saves");
        // Version 12 as it shipped: the same layout without the column.
        store.conn.execute_batch("ALTER TABLE sessions DROP COLUMN done_unseen;").expect("drops");
        store.conn.pragma_update(None, "user_version", 12).expect("version");
    }
    let mut store = Store::open_at(&dir).expect("upgrades");
    assert!(store.recovered_from().is_none(), "upgraded, not moved aside");
    assert_eq!(version(&store.conn), 13);
    let windows = store.load().expect("loads");
    assert_eq!(windows[0].sessions.len(), 2);
    assert!(windows[0].sessions.iter().all(|s| !s.done_unseen));
    let mut record = windows[0].clone();
    record.sessions[0].done_unseen = true;
    store.save_window(&record).expect("the column works");
    assert!(store.load().expect("loads")[0].sessions[0].done_unseen);
}
