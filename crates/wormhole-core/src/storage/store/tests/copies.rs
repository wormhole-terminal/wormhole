//! Old copies of the database are deleted at launch, except the newest one before an upgrade
//! while it is recent.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::super::copies::{KEEP_FOR, prune};
use super::*;

const NOW: u64 = 1_800_000_000;
const DAY: u64 = 24 * 60 * 60;

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("lists")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn the_newest_copy_before_an_upgrade_is_kept_for_two_weeks_and_the_rest_goes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let at = |days_ago: u64| NOW - days_ago * DAY;
    let files = [
        // Before upgrades: the newest (two in one second: `-2` is the later) and older ones.
        format!("wormhole.db.v7-{}", at(20)),
        format!("wormhole.db.v8-{}", at(3)),
        format!("wormhole.db.v8-{}-2", at(3)),
        // Set aside: one recent with its WAL, one old with its WAL, the codename's.
        format!("wormhole.db.corrupt-{}", at(1)),
        format!("wormhole.db.corrupt-{}-wal", at(1)),
        format!("wormhole.db.old-{}", at(30)),
        format!("wormhole.db.old-{}-wal", at(30)),
        format!("andromeda.db.old-{}", at(15)),
        format!("andromeda.db.v6-{}", at(16)),
        // Not copies, or not Wormhole's: never touched.
        "wormhole.db".into(),
        "wormhole.db-wal".into(),
        "config.json".into(),
        format!("wormhole.db.backup-{}", at(40)),
        format!("my-wormhole.db.old-{}", at(40)),
        format!("wormhole.db.old-{}.txt", at(40)),
    ];
    for name in &files {
        std::fs::write(dir.path().join(name), "copy").expect("writes");
    }
    // A folder by a copy's name is left alone too.
    std::fs::create_dir(dir.path().join(format!("wormhole.db.old-{}", at(50)))).expect("folder");

    let removed = prune(dir.path(), UNIX_EPOCH + Duration::from_secs(NOW));
    assert_eq!(removed.len(), 6, "{removed:?}");
    assert_eq!(
        names(dir.path()),
        [
            "config.json".to_string(),
            format!("my-wormhole.db.old-{}", at(40)),
            "wormhole.db".into(),
            "wormhole.db-wal".into(),
            format!("wormhole.db.backup-{}", at(40)),
            format!("wormhole.db.corrupt-{}", at(1)),
            format!("wormhole.db.corrupt-{}-wal", at(1)),
            format!("wormhole.db.old-{}", at(50)),
            format!("wormhole.db.old-{}.txt", at(40)),
            format!("wormhole.db.v8-{}-2", at(3)),
        ]
    );

    // Two weeks after it was made, the last one goes too.
    let later = UNIX_EPOCH + Duration::from_secs(at(3)) + KEEP_FOR;
    prune(dir.path(), later);
    assert!(!names(dir.path()).contains(&format!("wormhole.db.v8-{}-2", at(3))));
}

#[test]
fn the_renames_copy_goes_two_weeks_after_it_was_last_written() {
    let dir = tempfile::tempdir().expect("temp dir");
    for suffix in ["", "-wal", "-shm"] {
        std::fs::write(dir.path().join(format!("andromeda-before-wormhole.db{suffix}")), "x")
            .expect("writes");
    }
    let now = SystemTime::now();
    assert!(prune(dir.path(), now).is_empty(), "made just now: kept");
    assert_eq!(prune(dir.path(), now + KEEP_FOR + Duration::from_secs(60)).len(), 3);
    assert!(names(dir.path()).is_empty());
}

#[test]
fn a_link_named_like_a_copy_is_not_followed_or_deleted() {
    let dir = tempfile::tempdir().expect("temp dir");
    let elsewhere = tempfile::tempdir().expect("temp dir");
    let victim = elsewhere.path().join("keep.txt");
    std::fs::write(&victim, "not ours").expect("writes");
    let link = dir.path().join("wormhole.db.old-5");
    std::os::unix::fs::symlink(&victim, &link).expect("links");
    assert!(prune(dir.path(), SystemTime::now()).is_empty());
    assert!(victim.exists() && std::fs::symlink_metadata(&link).is_ok());
}

#[test]
fn opening_the_store_prunes_but_keeps_what_it_just_set_aside() {
    let dir = temp_dir("prune-on-open");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("wormhole.db.old-1000"), "ancient").expect("writes");
    std::fs::write(dir.join("wormhole.db"), "not a database, ".repeat(100)).expect("writes");
    let store = Store::open_at(&dir).expect("opens");
    assert!(!dir.join("wormhole.db.old-1000").exists());
    assert!(store.recovered_from().expect("recovered").exists());
}
