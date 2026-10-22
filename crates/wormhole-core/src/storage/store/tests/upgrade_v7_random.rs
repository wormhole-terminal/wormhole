//! The step from schema 6 to 7 against a model of it written the plain way: databases the app could
//! have written (splitting, opening and closing tabs and panes, moving a pane to a tab of its own, in
//! any order), upgraded and checked row by row. `upgrade_v7` has the cases by name; this one is for
//! the ones nobody thought of.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::migrations::empty_v6_database;
use super::*;

/// A small deterministic generator (xorshift64*), so a failing seed can be run again.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0..n (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// What one session of a version 6 database holds.
struct OldSession {
    id: i64,
    pinned: bool,
    active_tab: usize,
    /// Tabs in order, each its key and its panes' ids in order.
    tabs: Vec<(i64, Vec<i64>)>,
    /// A layout (panes in reading order, focused, zoomed) per tab with several panes.
    layouts: HashMap<i64, (Vec<i64>, i64, Option<i64>)>,
    notes: usize,
    tasks: usize,
}

struct OldWindow {
    id: i64,
    active: usize,
    sessions: Vec<OldSession>,
}

fn layout_json(ids: &[i64], focused: i64, zoomed: Option<i64>) -> String {
    let leaves: Vec<String> =
        ids.iter().map(|id| format!(r#"{{"ratio":0.25,"node":{{"leaf":{id}}}}}"#)).collect();
    let zoomed = zoomed.map_or("null".to_string(), |id| id.to_string());
    format!(
        r#"{{"tree":{{"root":{{"split":{{"axis":"row","children":[{}]}}}}}},"focused":{focused},"zoomed":{zoomed}}}"#,
        leaves.join(",")
    )
}

fn leaves(layout: &str) -> Vec<i64> {
    fn walk(node: &Value, out: &mut Vec<i64>) {
        if let Some(id) = node.get("leaf") {
            out.extend(id.as_i64());
        } else if let Some(children) = node["split"]["children"].as_array() {
            for child in children {
                walk(&child["node"], out);
            }
        }
    }
    let json: Value = serde_json::from_str(layout).expect("a layout is JSON");
    let mut out = Vec::new();
    walk(&json["tree"]["root"], &mut out);
    out
}

/// A session the way the app leaves it after a few things were done to it. Ids come from one
/// counter, as they do in the app.
fn random_session(rng: &mut Rng, next: &mut i64) -> OldSession {
    let mut take = || {
        *next += 1;
        *next
    };
    let id = take();
    let mut tabs = vec![(id, vec![id])];
    for _ in 0..rng.below(8) {
        match rng.below(5) {
            // Split a tab: a pane at its end.
            0 | 1 => {
                let tab = rng.below(tabs.len());
                tabs[tab].1.push(take());
            }
            // A new tab, its key its pane's id.
            2 => {
                let pane = take();
                tabs.push((pane, vec![pane]));
            }
            // Close a pane of a tab that has several, or a tab of a session that has several.
            3 => {
                let tab = rng.below(tabs.len());
                if tabs[tab].1.len() > 1 {
                    let pane = rng.below(tabs[tab].1.len());
                    tabs[tab].1.remove(pane);
                } else if tabs.len() > 1 {
                    tabs.remove(tab);
                }
            }
            // Move a pane out of its tab into one of its own, right after it, with a key of its own.
            _ => {
                let tab = rng.below(tabs.len());
                if tabs[tab].1.len() > 1 {
                    let pane = rng.below(tabs[tab].1.len());
                    let moved = tabs[tab].1.remove(pane);
                    let key = take();
                    tabs.insert(tab + 1, (key, vec![moved]));
                }
            }
        }
    }
    let mut layouts = HashMap::new();
    for (key, panes) in &tabs {
        if panes.len() > 1 {
            let mut order = panes.clone();
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i + 1));
            }
            // Mostly a pane the tab has; now and then one that closed (the app repairs it).
            let focused = if rng.chance(10) { 9999 } else { panes[rng.below(panes.len())] };
            let zoomed = rng.chance(20).then(|| panes[rng.below(panes.len())]);
            layouts.insert(*key, (order, focused, zoomed));
        }
    }
    OldSession {
        id,
        pinned: rng.chance(40),
        active_tab: rng.below(tabs.len()),
        tabs,
        layouts,
        notes: rng.below(3),
        tasks: rng.below(3),
    }
}

fn random_windows(rng: &mut Rng) -> Vec<OldWindow> {
    let mut next = 0;
    (1..=1 + rng.below(3) as i64)
        .map(|id| {
            let sessions: Vec<OldSession> =
                (0..1 + rng.below(5)).map(|_| random_session(rng, &mut next)).collect();
            OldWindow { id, active: rng.below(sessions.len()), sessions }
        })
        .collect()
}

fn write_v6(dir: &std::path::Path, windows: &[OldWindow], rng: &mut Rng) {
    let conn = empty_v6_database(dir);
    for (w, window) in windows.iter().enumerate() {
        conn.execute(
            "INSERT INTO windows (id, position, active) VALUES (?1, ?2, ?3)",
            (window.id, w as i64, window.active as i64),
        )
        .expect("window");
        let mut position = 0;
        for session in &window.sessions {
            position += 1 + rng.below(3) as i64;
            conn.execute(
                "INSERT INTO sessions (id, window_id, position, name, active_tab, pinned)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    session.id,
                    window.id,
                    position,
                    format!("s{}", session.id),
                    session.active_tab as i64,
                    session.pinned,
                ),
            )
            .expect("session");
            let mut pane_position = 0;
            for (key, panes) in &session.tabs {
                for (i, pane) in panes.iter().enumerate() {
                    let layout = (i == 0)
                        .then(|| session.layouts.get(key))
                        .flatten()
                        .map(|(order, focused, zoomed)| layout_json(order, *focused, *zoomed));
                    conn.execute(
                        "INSERT INTO tabs (id, session_id, tab_id, position, cwd, layout)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        (pane, session.id, key, pane_position, format!("p{pane}"), layout),
                    )
                    .expect("pane");
                    conn.execute(
                        "INSERT INTO scrollback (tab_id, hash, text) VALUES (?1, ?1, ?2)",
                        (pane, format!("out-{pane}")),
                    )
                    .expect("scrollback");
                    pane_position += 1;
                }
            }
            for k in 0..session.notes {
                conn.execute(
                    "INSERT INTO notes (session_id, position, body) VALUES (?1, ?2, ?3)",
                    (session.id, k as i64, format!("note {} {k}", session.id)),
                )
                .expect("note");
            }
            for k in 0..session.tasks {
                conn.execute(
                    "INSERT INTO tasks (session_id, position, text) VALUES (?1, ?2, ?3)",
                    (session.id, k as i64, format!("task {} {k}", session.id)),
                )
                .expect("task");
            }
        }
    }
}

/// One expected row of a window after the upgrade.
struct Row {
    old_pane: i64,
    new_id: i64,
    session: i64,
    first: bool,
    /// The tab (its key) when it has several panes: the row is a member of that tab's split.
    split_of: Option<i64>,
}

fn expected_rows(window: &OldWindow) -> Vec<Row> {
    let mut rows = Vec::new();
    for session in &window.sessions {
        let flat: Vec<(i64, i64, usize)> = session
            .tabs
            .iter()
            .flat_map(|(key, panes)| panes.iter().map(move |pane| (*pane, *key, panes.len())))
            .collect();
        let first = flat[0].0;
        for (i, (pane, key, size)) in flat.iter().enumerate() {
            let new_id = if i == 0 {
                session.id
            } else if *pane == session.id {
                first
            } else {
                *pane
            };
            rows.push(Row {
                old_pane: *pane,
                new_id,
                session: session.id,
                first: i == 0,
                split_of: (*size > 1).then_some(*key),
            });
        }
    }
    rows
}

/// Which of the shapes that need care a database had.
#[derive(Default)]
struct Seen {
    /// A session whose first pane is not the session's id (the pane renamed to it).
    renamed: usize,
    /// A session whose id is a pane that is not its first (the two swap).
    swapped: usize,
    splits: usize,
}

fn check(seed: u64) -> Seen {
    let mut seen = Seen::default();
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let windows = random_windows(&mut rng);
    let dir = temp_dir(&format!("v7-random-{seed}"));
    write_v6(&dir, &windows, &mut rng);
    let store = Store::open_at(&dir).unwrap_or_else(|err| panic!("seed {seed}: the upgrade failed: {err:#}"));
    let loaded = store.load().expect("loads");
    let fail = |what: &str| -> String { format!("seed {seed}: {what}") };

    let panes: usize =
        windows.iter().flat_map(|w| &w.sessions).flat_map(|s| &s.tabs).map(|t| t.1.len()).sum();
    assert_eq!(count(&store, "SELECT COUNT(*) FROM sessions") as usize, panes, "{}", fail("sessions"));
    assert_eq!(count(&store, "SELECT COUNT(*) FROM tabs") as usize, panes, "{}", fail("tabs"));
    assert_eq!(count(&store, "SELECT COUNT(*) FROM scrollback") as usize, panes, "{}", fail("scrollback"));
    assert_eq!(
        count(&store, "SELECT COUNT(*) FROM tabs WHERE id = session_id AND id = tab_id AND position = 0"),
        panes as i64,
        "{}",
        fail("a row per session with its id")
    );
    assert_eq!(count(&store, "SELECT COUNT(*) FROM pragma_foreign_key_check"), 0, "{}", fail("keys"));

    let mut split_ids = HashSet::new();
    let mut max_id = 0;
    for window in &windows {
        let rows = expected_rows(window);
        seen.renamed += rows.iter().filter(|r| r.first && r.old_pane != r.session).count();
        seen.swapped += rows.iter().filter(|r| !r.first && r.old_pane == r.session).count();
        let got = loaded.iter().find(|w| w.id == window.id).expect("the window");
        assert_eq!(
            got.sessions.iter().map(|s| s.id).collect::<Vec<_>>(),
            rows.iter().map(|r| r.new_id).collect::<Vec<_>>(),
            "{}",
            fail("the sessions of a window, in order")
        );
        // The old pane's id decides what a row must carry.
        let new_of: HashMap<i64, i64> = rows.iter().map(|r| (r.old_pane, r.new_id)).collect();
        let mut splits: HashMap<(i64, i64), i64> = HashMap::new();
        for (row, session) in rows.iter().zip(&got.sessions) {
            let old = windows
                .iter()
                .flat_map(|w| &w.sessions)
                .find(|s| s.id == row.session)
                .expect("the old session");
            max_id = max_id.max(row.new_id);
            let why = |what: &str| fail(&format!("session {} (pane {}): {what}", row.new_id, row.old_pane));
            assert_eq!(session.cwd.as_deref(), Some(format!("p{}", row.old_pane).as_str()), "{}", why("cwd"));
            assert_eq!(
                session.scrollback.as_deref(),
                Some(format!("out-{}", row.old_pane).as_str()),
                "{}",
                why("scrollback")
            );
            assert_eq!(session.pinned, old.pinned, "{}", why("pinned"));
            let name = row.first.then(|| format!("s{}", old.id));
            assert_eq!(session.name, name, "{}", why("name"));
            let (notes, tasks) = if row.first { (old.notes, old.tasks) } else { (0, 0) };
            assert_eq!(
                (session.notes.len(), session.tasks.len()),
                (notes, tasks),
                "{}",
                why("notes and tasks")
            );
            assert_eq!(session.split_id.is_some(), row.split_of.is_some(), "{}", why("split"));
            if let (Some(key), Some(split)) = (row.split_of, session.split_id) {
                assert_eq!(
                    *splits.entry((row.session, key)).or_insert(split),
                    split,
                    "{}",
                    why("same split")
                );
            }
        }
        // One split for each tab with several panes, each with its own id, and its layout.
        assert_eq!(got.splits.len(), splits.len(), "{}", fail("the splits of a window"));
        for ((session, key), split) in &splits {
            assert!(split_ids.insert(*split), "{}", fail("split ids differ"));
            let old = window.sessions.iter().find(|s| s.id == *session).expect("session");
            let (order, focused, zoomed) = &old.layouts[key];
            let layout = got
                .splits
                .iter()
                .find(|s| s.id == *split)
                .and_then(|s| s.layout.clone())
                .unwrap_or_else(|| panic!("{}", fail("a split has its layout")));
            let renamed = |id: &i64| new_of.get(id).copied().unwrap_or(*id);
            assert_eq!(
                leaves(&layout),
                order.iter().map(renamed).collect::<Vec<_>>(),
                "{}",
                fail("the layout's panes")
            );
            let json: Value = serde_json::from_str(&layout).expect("json");
            assert_eq!(json["focused"].as_i64(), Some(renamed(focused)), "{}", fail("focused"));
            assert_eq!(json["zoomed"].as_i64(), zoomed.as_ref().map(renamed), "{}", fail("zoomed"));
        }

        // The selected session: the focused pane of the old selected tab, else that tab's first.
        let old = &window.sessions[window.active];
        let (key, panes) = &old.tabs[old.active_tab.min(old.tabs.len() - 1)];
        let focused = old.layouts.get(key).map(|l| l.1).filter(|f| panes.contains(f)).unwrap_or(panes[0]);
        assert_eq!(
            got.sessions[got.active].cwd.as_deref(),
            Some(format!("p{focused}").as_str()),
            "{}",
            fail("active")
        );
    }
    let next = store.next_ids().expect("ids").0;
    assert!(next > max_id, "{}", fail("new ids are clear of the old ones"));
    assert!(
        split_ids.iter().all(|id| *id < next && *id > max_id),
        "{}",
        fail("split ids are past the others")
    );
    seen.splits = split_ids.len();
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
    seen
}

#[test]
fn a_few_hundred_databases_the_app_could_have_written_are_upgraded_as_the_model_says() {
    let mut seen = Seen::default();
    for seed in 1..=300 {
        let one = check(seed);
        seen.renamed += one.renamed;
        seen.swapped += one.swapped;
        seen.splits += one.splits;
    }
    // The generator does reach the shapes that need care.
    let (renamed, swapped, splits) = (seen.renamed, seen.swapped, seen.splits);
    assert!(renamed > 50 && swapped > 10 && splits > 100, "{renamed} {swapped} {splits}");
}
