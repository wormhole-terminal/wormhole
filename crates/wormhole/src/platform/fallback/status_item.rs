//! `platform::status_item` where the system has no status item: none is installed.

use smol::channel::Sender;

use crate::menubar::{MenuCommand, MenuSnapshot, StatusColors};

pub struct MenuBar;

impl MenuBar {
    pub fn install(_tx: Sender<MenuCommand>) -> Option<Self> {
        None
    }

    pub fn update(&mut self, _snapshot: &MenuSnapshot) {}

    pub fn show_frame(&mut self, _frame: Option<usize>) {}
}

#[cfg(feature = "devctl")]
pub fn save_icons(
    _dir: &std::path::Path,
    _prefix: &str,
    _colors: StatusColors,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    anyhow::bail!("no status item off macOS")
}
