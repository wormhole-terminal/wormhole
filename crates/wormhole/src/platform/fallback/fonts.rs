//! `platform::fonts` where the system can't be asked: no traits known (the bundled and custom
//! fonts still classify from their files), and no user font files loaded.

use std::path::Path;

use crate::platform::fonts::Traits;

pub fn classify(_family: &str) -> Option<Traits> {
    None
}

pub fn family_of_postscript(_postscript: &str) -> Option<String> {
    None
}

pub fn register(_path: &Path, _register: bool) -> anyhow::Result<()> {
    anyhow::bail!("custom fonts need macOS")
}
