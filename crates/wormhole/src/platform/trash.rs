//! Moving a file to the Trash, where the person can take it back: a theme they imported and no
//! longer want. Tests never touch the real Trash.

use std::path::Path;

use super::os::trash as os;

/// Move `path` to the Trash.
pub fn move_to_trash(path: &Path) -> anyhow::Result<()> {
    if cfg!(test) {
        anyhow::bail!("tests don't use the Trash");
    }
    os::move_to_trash(path)
}
