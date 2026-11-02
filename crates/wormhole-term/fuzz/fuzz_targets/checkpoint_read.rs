#![no_main]

//! Arbitrary bytes as the files of a checkpoint directory, as a restore would find them after a crash, a
//! bug or a hostile program with the user's rights: the first byte says how to split the rest between a
//! manifest and a screen. Reading must not panic or hang, a screen that comes back must be one that opened
//! (so only the real one), and the replay filter must only let text and SGR through.

use libfuzzer_sys::fuzz_target;
use wormhole_term::checkpoint::{Key, Manifest, Snapshot, Store, read_latest, safe_replay};

const ID: &str = "abcdefghij12";

fuzz_target!(|data: &[u8]| {
    let Some((&split, rest)) = data.split_first() else { return };
    let at = (usize::from(split) * rest.len()) / 256;
    let (manifest, screen) = rest.split_at(at);

    let _ = Manifest::parse(&String::from_utf8_lossy(manifest));
    let safe = safe_replay(screen);
    assert!(safe.iter().all(|b| matches!(*b, b'\n' | b'\r' | b'\t' | 0x1b | 0x20..=0x7e | 0x80..=0xff)));

    let root = tempfile::tempdir().expect("a temp dir");
    let key = Key::new([7; 32]);
    let mut store = Store::open(root.path(), ID).expect("a store");
    let snapshot = Snapshot { time: 1, cols: 80, rows: 24, cwd: None, shell: None, title: None, screen: Some(b"real") };
    store.write(Some(&key), &snapshot).expect("a write");
    std::fs::write(store.dir().join("g000000000001.manifest"), manifest).expect("a manifest");
    std::fs::write(store.dir().join("g000000000001.screen"), screen).expect("a screen");
    if let Ok(read) = read_latest(store.dir(), ID, Some(&key)) {
        if let Some(opened) = read.screen {
            assert_eq!(opened.as_slice(), b"real");
        }
    }
});
