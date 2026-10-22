//! `platform::apps` where no app can be found by a bundle identifier: nothing is installed
//! (Finder is still offered by the app's table), and nothing can be launched.

use std::path::{Path, PathBuf};
use std::process::Command;

use wormhole_core::open_file::Launch;

pub fn app_path(_: &str) -> Option<PathBuf> {
    None
}

pub fn bundle_of(_: &Path) -> Option<String> {
    None
}

pub fn icon_png(_: &str, _: usize) -> Option<Vec<u8>> {
    None
}

pub fn command(_: &str, _: Launch) -> Option<Command> {
    None
}

pub fn text_editor_command(_: &Path) -> Option<Command> {
    None
}
