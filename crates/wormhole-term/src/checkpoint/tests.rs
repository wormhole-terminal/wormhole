//! The checkpoint directory against real files: writing, reading, the crashes between the steps of a write,
//! a damaged or foreign generation, another key, a newer format, and hostile input.

use std::fs;
use std::path::Path;

use super::seal::{Binding, KEY_LEN};
use super::store::{Issue, KEEP, ReadError, Snapshot, Store, forget, read_latest};
use super::*;

const ID: &str = "abcdefghij12";

fn key(byte: u8) -> Key {
    Key::new([byte; KEY_LEN])
}

fn snapshot<'a>(screen: Option<&'a [u8]>, title: &'a str) -> Snapshot<'a> {
    Snapshot {
        time: 1_760_000_000,
        cols: 80,
        rows: 24,
        cwd: Some("/Users/me/src"),
        shell: Some("login"),
        title: Some(title),
        ended: None,
        screen,
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn sessions() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temp dir")
}

#[test]
fn a_checkpoint_comes_back_with_its_manifest_and_screen() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    let written = store.write(Some(&a), &snapshot(Some(b"\x1b[1mhello\x1b[0m\r\n"), "t")).unwrap();
    assert_eq!((written.generation, written.screen), (1, true));
    let read = read_latest(store.dir(), ID, Some(&a)).unwrap();
    assert_eq!(read.manifest.cwd.as_deref(), Some("/Users/me/src"));
    assert_eq!((read.manifest.cols, read.manifest.rows, read.manifest.generation), (80, 24, 1));
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"\x1b[1mhello\x1b[0m\r\n"[..]));
    assert_eq!(read.issue, None);
}

#[test]
fn the_directory_is_private_and_the_screen_is_not_in_it_in_the_clear() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    store.write(Some(&key(1)), &snapshot(Some(b"password hunter2\r\n"), "t")).unwrap();
    let dir = store.dir().to_path_buf();
    assert_eq!(crate::sys::fs::mode(&fs::metadata(&dir).unwrap()) & 0o777, 0o700);
    for name in names(&dir) {
        let path = dir.join(&name);
        assert_eq!(crate::sys::fs::mode(&fs::metadata(&path).unwrap()) & 0o777, 0o600, "{name}");
        assert!(!fs::read(&path).unwrap().windows(7).any(|w| w == b"hunter2"), "{name} holds the text");
    }
}

#[test]
fn without_a_key_only_the_manifest_is_written_and_never_a_screen() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let written = store.write(None, &snapshot(Some(b"secret screen"), "t")).unwrap();
    assert!(!written.screen);
    assert_eq!(names(store.dir()), ["g000000000001.manifest"]);
    for name in names(store.dir()) {
        assert!(!fs::read(store.dir().join(name)).unwrap().windows(6).any(|w| w == b"secret"));
    }
    let read = read_latest(store.dir(), ID, None).unwrap();
    assert!(read.screen.is_none() && read.issue.is_none());
    assert_eq!(
        read.manifest.cwd.as_deref(),
        Some("/Users/me/src"),
        "the folder and the shell are still there"
    );
}

#[test]
fn the_newest_two_generations_are_kept() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    for i in 0..5 {
        store.write(Some(&a), &snapshot(Some(format!("screen {i}").as_bytes()), "t")).unwrap();
    }
    assert_eq!(KEEP, 2);
    assert_eq!(
        names(store.dir()),
        ["g000000000004.manifest", "g000000000004.screen", "g000000000005.manifest", "g000000000005.screen"]
    );
    let read = read_latest(store.dir(), ID, Some(&a)).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"screen 4"[..]));
}

#[test]
fn a_new_store_goes_on_from_the_generation_it_finds() {
    let root = sessions();
    let a = key(1);
    Store::open(root.path(), ID).unwrap().write(Some(&a), &snapshot(Some(b"one"), "t")).unwrap();
    let mut again = Store::open(root.path(), ID).unwrap();
    assert_eq!(again.write(Some(&a), &snapshot(Some(b"two"), "t")).unwrap().generation, 2);
}

#[test]
fn a_write_killed_between_its_steps_leaves_the_last_generation_whole() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"good"), "t")).unwrap();
    let dir = store.dir().to_path_buf();
    // Killed after the screen was renamed in and before the manifest: a screen with no manifest.
    fs::write(dir.join("g000000000002.screen"), b"half a generation").unwrap();
    // Killed while writing a temporary file.
    fs::write(dir.join(".g000000000002.manifest.99999.tmp"), b"cut").unwrap();
    let read = read_latest(&dir, ID, Some(&a)).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"good"[..]), "the whole generation wins");
    // The next write cleans both up and does not reuse the dead generation's number.
    let mut next = Store::open(root.path(), ID).unwrap();
    let written = next.write(Some(&a), &snapshot(Some(b"later"), "t")).unwrap();
    assert_eq!(written.generation, 3);
    assert!(
        !names(&dir).iter().any(|n| n.ends_with(".tmp") || n == "g000000000002.screen"),
        "{:?}",
        names(&dir)
    );
}

#[test]
fn a_damaged_newest_generation_gives_way_to_the_one_before() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"older"), "t")).unwrap();
    store.write(Some(&a), &snapshot(Some(b"newer"), "t")).unwrap();
    let dir = store.dir().to_path_buf();
    for damage in ["flip", "cut", "gone"] {
        let path = dir.join("g000000000002.screen");
        let original = fs::read(&path).unwrap_or_default();
        match damage {
            "flip" => {
                let mut bytes = original.clone();
                let last = bytes.len() - 1;
                bytes[last] ^= 1;
                fs::write(&path, bytes).unwrap();
            }
            "cut" => fs::write(&path, &original[..original.len() / 2]).unwrap(),
            _ => fs::remove_file(&path).unwrap(),
        }
        let read = read_latest(&dir, ID, Some(&a)).unwrap();
        assert_eq!(read.manifest.generation, 1, "{damage}");
        assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"older"[..]), "{damage}");
        fs::write(&path, &original).unwrap();
    }
}

#[test]
fn a_damaged_manifest_is_skipped_too() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"older"), "t")).unwrap();
    store.write(Some(&a), &snapshot(Some(b"newer"), "t")).unwrap();
    fs::write(store.dir().join("g000000000002.manifest"), b"\xff\xfe not a manifest").unwrap();
    assert_eq!(read_latest(store.dir(), ID, Some(&a)).unwrap().manifest.generation, 1);
}

#[test]
fn another_key_gives_the_manifest_and_says_why_there_is_no_screen() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    store.write(Some(&key(1)), &snapshot(Some(b"sealed"), "t")).unwrap();
    let wrong = read_latest(store.dir(), ID, Some(&key(2))).unwrap();
    assert!(wrong.screen.is_none());
    assert_eq!(wrong.issue, Some(Issue::WrongKey));
    assert_eq!(wrong.manifest.cwd.as_deref(), Some("/Users/me/src"));
    let none = read_latest(store.dir(), ID, None).unwrap();
    assert_eq!(none.issue, Some(Issue::NoKey));
}

#[test]
fn a_screen_sealed_for_another_terminal_or_generation_does_not_open_by_being_renamed() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"one"), "t")).unwrap();
    store.write(Some(&a), &snapshot(Some(b"two"), "t")).unwrap();
    let dir = store.dir().to_path_buf();
    // Generation 1's screen put where generation 2's is.
    fs::copy(dir.join("g000000000001.screen"), dir.join("g000000000002.screen")).unwrap();
    let read = read_latest(&dir, ID, Some(&a)).unwrap();
    assert_eq!(
        read.screen.as_deref().map(Vec::as_slice),
        Some(&b"one"[..]),
        "the swapped file is damaged, the real older one opens"
    );
    assert_eq!(read.manifest.generation, 1);
}

#[test]
fn a_newer_format_is_left_alone_and_a_missing_directory_is_missing() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"one"), "t")).unwrap();
    let path = store.dir().join("g000000000001.manifest");
    let text = fs::read_to_string(&path).unwrap().replace("min_reader=1", "min_reader=9");
    fs::write(&path, text).unwrap();
    assert_eq!(read_latest(store.dir(), ID, Some(&a)).unwrap_err(), ReadError::Newer(9));
    assert!(path.exists(), "reading changed nothing");
    assert_eq!(read_latest(&root.path().join("nobody"), ID, Some(&a)).unwrap_err(), ReadError::Missing);
    let empty = root.path().join("empty");
    fs::create_dir(&empty).unwrap();
    assert_eq!(read_latest(&empty, ID, Some(&a)).unwrap_err(), ReadError::Missing, "an empty dir");
}

#[test]
fn nothing_readable_is_unusable() {
    let root = sessions();
    let store = Store::open(root.path(), ID).unwrap();
    fs::write(store.dir().join("g000000000001.manifest"), b"garbage").unwrap();
    assert_eq!(read_latest(store.dir(), ID, Some(&key(1))).unwrap_err(), ReadError::Unusable);
}

#[test]
fn forgetting_removes_the_files_and_the_directory() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    store.write(Some(&key(1)), &snapshot(Some(b"x"), "t")).unwrap();
    let dir = store.dir().to_path_buf();
    forget(&dir);
    assert!(!dir.exists());
    forget(&dir); // already gone: fine
}

#[test]
fn a_directory_removed_under_a_running_store_comes_back_with_the_next_write() {
    let root = sessions();
    let mut store = Store::open(root.path(), ID).unwrap();
    let a = key(1);
    store.write(Some(&a), &snapshot(Some(b"one"), "t")).unwrap();
    forget(store.dir());
    assert!(!store.dir().exists());
    let written = store.write(Some(&a), &snapshot(Some(b"two"), "t")).unwrap();
    assert_eq!(written.generation, 2);
    let read = read_latest(store.dir(), ID, Some(&a)).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"two"[..]));
    assert_eq!(crate::sys::fs::mode(&fs::metadata(store.dir()).unwrap()) & 0o777, 0o700);
}

#[test]
fn only_a_host_id_names_a_directory() {
    let root = sessions();
    for bad in ["", "short", "../etc/passwd1", "ABCDEFGHIJ12", "abcdefghij1/"] {
        assert!(Store::open(root.path(), bad).is_err(), "{bad:?}");
    }
}

/// The sealed screen of format 1 for a fixed key, nonce and binding: a change that alters these bytes is a
/// new format.
#[test]
fn the_sealed_screen_of_format_1_is_these_bytes() {
    let key = Key::new(std::array::from_fn(|i| i as u8));
    let binding = Binding { id: ID, generation: 7, key_id: key.id() };
    let sealed = seal::seal_with(&key, &binding, b"\x1b[1mhello\x1b[0m\r\n", [9u8; 12]);
    let hex: String = sealed.iter().map(|b| format!("{b:02x}")).collect();
    let golden = include_str!("../../tests/vectors/checkpoint-screen-1.hex").trim();
    assert_eq!(hex, golden);
    let opened = seal::open(&key, &binding, &sealed).unwrap();
    assert_eq!(&**opened, b"\x1b[1mhello\x1b[0m\r\n");
}

/// Whatever is on disk, reading never panics, and never returns a screen it did not open. A small
/// deterministic stand-in for the fuzz target (`crates/wormhole-term/fuzz`).
#[test]
fn hostile_files_never_panic_the_reader() {
    let root = sessions();
    let a = key(1);
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut store = Store::open(root.path(), ID).unwrap();
    store.write(Some(&a), &snapshot(Some(b"real"), "t")).unwrap();
    let good_manifest = fs::read(store.dir().join("g000000000001.manifest")).unwrap();
    let good_screen = fs::read(store.dir().join("g000000000001.screen")).unwrap();
    for round in 0..300 {
        let mutate = |base: &[u8], next: &mut dyn FnMut() -> u64| {
            let mut bytes = base.to_vec();
            for _ in 0..(next() % 6) {
                if bytes.is_empty() {
                    break;
                }
                let at = (next() as usize) % bytes.len();
                match next() % 3 {
                    0 => bytes[at] = next() as u8,
                    1 => bytes.truncate(at),
                    _ => bytes.insert(at, next() as u8),
                }
            }
            bytes
        };
        fs::write(store.dir().join("g000000000001.manifest"), mutate(&good_manifest, &mut next)).unwrap();
        fs::write(store.dir().join("g000000000001.screen"), mutate(&good_screen, &mut next)).unwrap();
        if let Ok(read) = read_latest(store.dir(), ID, Some(&a))
            && let Some(screen) = read.screen
        {
            assert_eq!(&**screen, b"real", "round {round}: a screen that opened is the one that was sealed");
        }
    }
    // The manifest parser and the replay filter on arbitrary bytes.
    for _ in 0..500 {
        let len = (next() % 200) as usize;
        let junk: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        let _ = Manifest::parse(&String::from_utf8_lossy(&junk));
        let safe = safe_replay(&junk);
        assert!(safe.iter().all(|b| *b == b'\n'
            || *b == b'\r'
            || *b == b'\t'
            || *b >= 0x20 && *b != 0x7f
            || *b == 0x1b));
    }
}
