//! Keys: a shortcut as text (`keyspec`), every action's keys, the menus and the shortcut sheet as one
//! table (`keymap`), and recording a shortcut in Settings (`shortcut_record`).
//!
//! The lowest tier: nothing here uses another domain of core.

pub mod keymap;
pub mod keyspec;
pub mod shortcut_record;
