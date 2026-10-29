//! The close guard's decision and the window's keys, without a window.
use wormhole_core::keys::keyspec::{KeyMods, KeySpec};

use super::title::SaveAction;
use super::{Closing, closing, save_key};

#[test]
fn a_window_without_edits_closes_at_once() {
    assert_eq!(closing(false, false, false), Closing::Now);
    assert_eq!(closing(false, true, false), Closing::Now);
}

#[test]
fn edits_are_asked_about_once() {
    assert_eq!(closing(true, false, false), Closing::Ask);
    assert_eq!(closing(true, true, false), Closing::AskedAlready);
}

#[test]
fn quitting_does_not_wait_for_an_answer() {
    assert_eq!(closing(true, false, true), Closing::Now);
    assert_eq!(closing(true, true, true), Closing::Now);
}

/// ⌘S saves and ⇧⌘S duplicates, as the key table's Theme Studio rows say.
#[test]
fn the_save_keys_are_the_tables() {
    let key = |cmd, shift, alt, k| KeySpec::new(KeyMods { cmd, shift, alt, ctrl: false }, k);
    assert_eq!(save_key(&key(true, false, false, "s")), Some(SaveAction::Save));
    assert_eq!(save_key(&key(true, true, false, "s")), Some(SaveAction::Duplicate));
    // A US keyboard reports ⇧⌘S as ⌘S with a capital.
    assert_eq!(save_key(&key(true, true, false, "S")), Some(SaveAction::Duplicate));
    // ⌥⌘S is Stash Prompt's, S alone is typing, and ⌘W is not a save.
    assert_eq!(save_key(&key(true, false, true, "s")), None);
    assert_eq!(save_key(&key(false, false, false, "s")), None);
    assert_eq!(save_key(&key(true, false, false, "w")), None);
}
