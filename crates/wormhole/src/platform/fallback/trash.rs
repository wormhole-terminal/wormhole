//! `platform::trash` without a Trash to move to.

use std::path::Path;

pub fn move_to_trash(path: &Path) -> anyhow::Result<()> {
    anyhow::bail!("no Trash here for {}", path.display())
}
