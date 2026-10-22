//! A message without variables, named by the constant the generator wrote for it.

use crate::generated::ids::IDS;

/// An index into the generated id list. Only `keys.rs` makes them, one per message (or attribute)
/// that takes no variables, so a `Key` that does not exist is a compile error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(u16);

impl Key {
    #[doc(hidden)]
    pub(crate) const fn new(index: u16) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }

    /// The Fluent id (`settings-theme-mode`, `settings-theme-mode.description`), for logs and tests.
    pub fn id(self) -> &'static str {
        IDS.get(self.index()).copied().unwrap_or("")
    }
}
