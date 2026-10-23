//! This machine's name.

use std::sync::OnceLock;

use super::unix::host as os;

/// This machine's host name, read once.
pub fn name() -> Option<&'static str> {
    static NAME: OnceLock<Option<String>> = OnceLock::new();
    NAME.get_or_init(os::read_name).as_deref()
}
