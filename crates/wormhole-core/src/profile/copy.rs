//! What a new profile takes from the one it starts as a copy of.
//!
//! Its settings (`config.json`: theme, fonts, keys, shells, notifications, hotkey …) and the
//! themes, fonts and icon themes the person added. Not its terminals (the database), not its
//! trusted projects (an approval is for the profile it was given in), and, in `ProfileList`, not its
//! folders or its agent logins.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use super::paths::ProfilePaths;
use crate::fsutil::{self, LinkPolicy};

/// Largest single file copied; a font family is a few MiB.
const MAX_FILE: u64 = 64 * 1024 * 1024;
/// Most files and folders copied, and how deep: a folder of fonts the person linked in is
/// recreated as a link, so these only stop a huge pile dropped in the folder itself.
const MAX_ENTRIES: usize = 20_000;
const MAX_DEPTH: usize = 8;

pub(super) fn copy_settings(from: &ProfilePaths, to: &ProfilePaths) -> io::Result<()> {
    fs::create_dir_all(to.dir())?;
    match fsutil::read_capped(&from.config_file(), 1024 * 1024, LinkPolicy::Follow) {
        Ok(bytes) => fsutil::write_atomic(&to.config_file(), &bytes, LinkPolicy::Follow)?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let mut budget = MAX_ENTRIES;
    for (src, dst) in [
        (from.themes_dir(), to.themes_dir()),
        (from.fonts_dir(), to.fonts_dir()),
        (from.icon_themes_dir(), to.icon_themes_dir()),
    ] {
        copy_tree(&src, &dst, 0, &mut budget)?;
    }
    Ok(())
}

/// `src` as `dst`: files copied, folders made, links recreated as links (never followed, so a link
/// in a folder can't pull in what it points at). Anything else (sockets, devices) is left out.
fn copy_tree(src: &Path, dst: &Path, depth: usize, budget: &mut usize) -> io::Result<()> {
    let meta = match fs::symlink_metadata(src) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if *budget == 0 || depth > MAX_DEPTH {
        return Ok(());
    }
    *budget -= 1;
    if meta.file_type().is_symlink() {
        let target = fs::read_link(src)?;
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        return match symlink(target, dst) {
            Err(err) if err.kind() != io::ErrorKind::AlreadyExists => Err(err),
            _ => Ok(()),
        };
    }
    if meta.is_dir() {
        fs::create_dir_all(dst)?;
        let mut names: Vec<_> = fs::read_dir(src)?.filter_map(Result::ok).map(|e| e.file_name()).collect();
        names.sort();
        for name in names {
            copy_tree(&src.join(&name), &dst.join(&name), depth + 1, budget)?;
        }
    } else if meta.is_file() {
        match fsutil::read_capped(src, MAX_FILE, LinkPolicy::Follow) {
            Ok(bytes) => fsutil::write_atomic(dst, &bytes, LinkPolicy::Follow)?,
            // One file that is too big or unreadable must not stop the profile from being made.
            Err(err) => tracing::warn!("not copying {} to a new profile: {err}", src.display()),
        }
    }
    Ok(())
}
