//! The app's actions and keys, the menu bar and shortcut recording from core's `keys` domain, live: Swift
//! reads the table instead of a dump of it. Everything here is pure and takes no lock of its own.
//!
//! Actions are named by text (`newTerminal`, `terminal.3`), as the Swift side's `ActionID` writes them.

pub mod keys;
pub mod lists;
pub mod menus;
pub mod records;

use records::KeymapEntryRecord;

use wormhole_core::keys::keymap;

/// Every action, in the table's order.
#[uniffi::export]
pub fn keymap_entries() -> Vec<KeymapEntryRecord> {
    keymap::entries().iter().map(KeymapEntryRecord::from).collect()
}

/// The actions hidden while the features and feature flags named are off (what the window puts in
/// `MenuState.hidden`): a row is tagged with its feature's key and its flag's key.
#[uniffi::export]
pub fn keymap_hidden_actions(features_off: Vec<String>, flags_off: Vec<String>) -> Vec<String> {
    let is_off = |tag: Option<&str>, off: &[String]| tag.is_some_and(|tag| off.iter().any(|o| o == tag));
    keymap::entries()
        .iter()
        .filter(|entry| is_off(entry.feature, &features_off) || is_off(entry.flag, &flags_off))
        .map(|entry| records::id_string(entry.id))
        .collect()
}

#[cfg(test)]
mod tests;
