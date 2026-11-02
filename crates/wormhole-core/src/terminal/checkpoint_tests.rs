//! The app's reading of a checkpoint and its looking after the folder, against real files written the way a
//! host writes them.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use wormhole_term::checkpoint::{Issue, Key, Snapshot, Store};

use super::*;

const A: &str = "aaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbb";
const C: &str = "cccccccccccc";

fn key(byte: u8) -> Key {
    Key::new([byte; 32])
}

fn words(time: u64) -> String {
    format!("restored at {time}")
}

fn write(sessions: &Path, id: &str, key: &Key, screen: &[u8], ended: Option<&str>, time: u64) {
    let mut store = Store::open(sessions, id).expect("a store");
    store.write(key, &Snapshot { time, ended, screen }).expect("written");
}

#[test]
fn a_checkpoint_is_read_as_a_screen_with_a_divider_that_says_when() {
    let sessions = tempfile::tempdir().unwrap();
    write(sessions.path(), A, &key(1), b"\x1b[1mhello\x1b[0m\r\nworld\r\n", None, 1_793_023_327);
    let seed = read(sessions.path(), A, Some(&key(1)), &words).expect("reads");
    assert!(seed.text.starts_with("\x1b[1mhello\x1b[0m\r\nworld\r\n"), "{:?}", seed.text);
    let divider_line = seed.text.lines().last().unwrap();
    assert_eq!(divider_line, "\x1b[2;3m──── restored at 1793023327 ────\x1b[0m");
    assert_eq!((seed.time, seed.ended.as_deref()), (1_793_023_327, None));
}

#[test]
fn a_program_that_ended_is_said_to_have() {
    let sessions = tempfile::tempdir().unwrap();
    write(sessions.path(), A, &key(1), b"bye\r\n", Some("exit:0"), 1);
    assert_eq!(read(sessions.path(), A, Some(&key(1)), &words).unwrap().ended.as_deref(), Some("exit:0"));
}

#[test]
fn whatever_is_in_the_screen_only_text_and_colors_get_in() {
    let sessions = tempfile::tempdir().unwrap();
    write(
        sessions.path(),
        A,
        &key(1),
        b"ok\x1b[6n\x1b]52;c;?\x07\x1b]0;t\x07\x1b[?1000h\x1b[31mred\x1b[0m\r\n",
        None,
        1,
    );
    let seed = read(sessions.path(), A, Some(&key(1)), &words).unwrap();
    assert!(!seed.text.contains("\x1b]") && !seed.text.contains("\x1b[6n") && !seed.text.contains("\x1b[?"));
    assert!(seed.text.contains("\x1b[31mred"));
}

#[test]
fn what_cannot_be_read_says_why_and_a_seed_shows_no_screen_in_a_log() {
    let sessions = tempfile::tempdir().unwrap();
    assert_eq!(read(sessions.path(), A, Some(&key(1)), &words), Err(Skipped::Missing));
    write(sessions.path(), A, &key(1), b"x\r\n", None, 1);
    assert_eq!(
        read(sessions.path(), A, Some(&key(2)), &words),
        Err(Skipped::NoScreen(Some(Issue::WrongKey)))
    );
    assert_eq!(read(sessions.path(), A, None, &words), Err(Skipped::NoScreen(Some(Issue::NoKey))));
    let seed = read(sessions.path(), A, Some(&key(1)), &words).unwrap();
    assert!(!format!("{seed:?}").contains('x') || format!("{seed:?}").contains("bytes"), "{seed:?}");
    assert!(!format!("{seed:?}").contains("restored at"), "no plaintext in the Debug");
    // A newer Wormhole's checkpoint is left as it is.
    let manifest = sessions.path().join(A).join("g000000000001.manifest");
    let newer = fs::read_to_string(&manifest).unwrap().replace("min_reader=1", "min_reader=7");
    fs::write(&manifest, newer).unwrap();
    assert_eq!(read(sessions.path(), A, Some(&key(1)), &words), Err(Skipped::Newer));
    // Nothing in it reads.
    fs::create_dir(sessions.path().join(C)).unwrap();
    fs::write(sessions.path().join(C).join("g000000000001.manifest"), b"junk").unwrap();
    assert_eq!(read(sessions.path(), C, Some(&key(1)), &words), Err(Skipped::Unusable));
}

#[test]
fn the_folder_is_private_and_kept_out_of_backups() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("sessions.noindex");
    let mut excluded = Vec::new();
    prepare_dir(&dir, |p| {
        excluded.push(p.to_path_buf());
        Ok(())
    })
    .unwrap();
    assert_eq!(excluded, std::slice::from_ref(&dir));
    assert_eq!(wormhole_term::sys::fs::mode(&fs::metadata(&dir).unwrap()) & 0o777, 0o700);
    // A folder that cannot be excluded still opens: that is a warning.
    let again = parent.path().join("again");
    prepare_dir(&again, |_| Err(io::Error::other("no"))).unwrap();
    assert!(again.is_dir());
}

#[test]
fn forgetting_removes_one_checkpoint_and_only_a_real_id_names_one() {
    let sessions = tempfile::tempdir().unwrap();
    write(sessions.path(), A, &key(1), b"a\r\n", None, 1);
    write(sessions.path(), B, &key(1), b"b\r\n", None, 1);
    forget(sessions.path(), A);
    assert!(!sessions.path().join(A).exists() && sessions.path().join(B).is_dir());
    forget(sessions.path(), "../b");
    forget(sessions.path(), "");
    assert!(sessions.path().join(B).is_dir());
}

fn age(sessions: &Path, id: &str, by: Duration, now: SystemTime) {
    for entry in fs::read_dir(sessions.join(id)).unwrap().flatten() {
        let file = fs::File::options().write(true).open(entry.path()).unwrap();
        file.set_modified(now - by).unwrap();
    }
}

#[test]
fn the_sweep_takes_the_old_and_spares_the_running_and_the_recent() {
    let sessions = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    for id in [A, B, C] {
        write(sessions.path(), id, &key(1), b"x\r\n", None, 1);
    }
    age(sessions.path(), A, Duration::from_secs(8 * 24 * 3600), now);
    age(sessions.path(), B, Duration::from_secs(9 * 24 * 3600), now);
    let running: HashSet<String> = [B.to_owned()].into();
    assert_eq!(sweep(sessions.path(), &running, now), [A.to_owned()]);
    assert!(!sessions.path().join(A).exists());
    assert!(sessions.path().join(B).is_dir(), "its host still runs");
    assert!(sessions.path().join(C).is_dir(), "a day old");
}

#[test]
fn over_the_budget_the_oldest_go_first() {
    let sessions = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    let big = vec![b'x'; 10_000];
    for id in [A, B, C] {
        write(sessions.path(), id, &key(1), &big, None, 1);
    }
    age(sessions.path(), A, Duration::from_secs(3 * 3600), now);
    age(sessions.path(), B, Duration::from_secs(2 * 3600), now);
    age(sessions.path(), C, Duration::from_secs(3600), now);
    let removed = sweep_with(sessions.path(), &HashSet::new(), now, MAX_AGE, 25_000);
    assert_eq!(removed, [A.to_owned()]);
    assert!(sessions.path().join(B).is_dir() && sessions.path().join(C).is_dir());
    assert!(sweep_with(sessions.path(), &HashSet::new(), now, MAX_AGE, 25_000).is_empty());
}

/// The feature is off everywhere: what it left goes, except the checkpoints of hosts that still run.
#[test]
fn with_the_feature_off_everything_but_running_hosts_goes() {
    let sessions = tempfile::tempdir().unwrap();
    for id in [A, B, C] {
        write(sessions.path(), id, &key(1), b"x\r\n", None, 1);
    }
    let running: HashSet<String> = [B.to_owned()].into();
    let mut removed = remove_unused(sessions.path(), &running);
    removed.sort();
    assert_eq!(removed, [A.to_owned(), C.to_owned()]);
    assert!(sessions.path().join(B).is_dir() && !sessions.path().join(A).exists());
    assert!(remove_unused(&sessions.path().join("nowhere"), &running).is_empty());
}

#[test]
fn a_terminal_is_restored_from_its_checkpoint_or_from_its_text_as_before() {
    let sessions = tempfile::tempdir().unwrap();
    write(sessions.path(), A, &key(1), b"\x1b[1mfrom the checkpoint\x1b[0m\r\n", None, 1_793_023_327);
    let text_only =
        || super::super::SpawnOptions { restore: Some("saved text".into()), ..Default::default() };
    let mut options = text_only();
    assert!(seed_options(sessions.path(), A, Some(&key(1)), &mut options, &words));
    let drawn = options.restore.as_deref().unwrap();
    assert!(drawn.contains("from the checkpoint") && drawn.contains("restored at 1793023327"));
    assert!(!drawn.contains("saved text"));
    // Another key, none, no checkpoint: the text stays.
    for (id, key_given) in [(A, Some(key(2))), (A, None), (B, Some(key(1)))] {
        let mut options = text_only();
        assert!(!seed_options(sessions.path(), id, key_given.as_ref(), &mut options, &words));
        assert_eq!(options.restore.as_deref(), Some("saved text"));
    }
}

/// The old host's checkpoint is spent only once the new host has written its own: until then another restart
/// or a crash must still find it.
#[test]
fn the_old_checkpoint_stays_until_the_new_hosts_has_landed() {
    let sessions = tempfile::tempdir().unwrap();
    write(sessions.path(), A, &key(1), b"the old screen\r\n", None, 1);
    assert!(!forget_when_confirmed(sessions.path(), A, B, &key(2)), "the new host wrote nothing yet");
    assert!(sessions.path().join(A).is_dir());
    fs::create_dir(sessions.path().join(B)).unwrap();
    assert!(!forget_when_confirmed(sessions.path(), A, B, &key(2)), "an empty directory is not a write");
    write(sessions.path(), B, &key(2), b"the new screen\r\n", None, 2);
    assert!(!forget_when_confirmed(sessions.path(), A, B, &key(3)), "not one that opens with this key");
    assert!(sessions.path().join(A).is_dir());
    assert!(forget_when_confirmed(sessions.path(), A, B, &key(2)));
    assert!(!sessions.path().join(A).exists() && sessions.path().join(B).is_dir());
}
