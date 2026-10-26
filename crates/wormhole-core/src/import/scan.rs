//! Finding the terminals that have settings on this Mac and reading each into a proposal.

use std::path::PathBuf;
use std::time::SystemTime;

use super::env::Env;
use super::proposal::Proposal;
use super::read::Reader;
use super::source::Source;
use super::{alacritty, ghostty, iterm, kitty, locations, terminal_app, vscode, warp, wezterm, zed};

/// How many files deep includes are followed (Alacritty's own limit is five).
const DEFAULT_DEPTH: usize = 8;

/// A source with settings on this Mac.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub source: Source,
    /// The files that were read, the settings file first.
    pub files: Vec<PathBuf>,
    /// When the newest of them was changed, to put the terminal in use first.
    pub modified: Option<SystemTime>,
    pub proposal: Proposal,
}

/// Every source that has a settings file under `env`, the one changed last first. A file that
/// cannot be read or understood never fails the scan: it is a [`Skipped`](super::Skipped) in its
/// source's proposal.
pub fn scan(env: &Env) -> Vec<Found> {
    let mut found: Vec<Found> = Source::ALL.into_iter().filter_map(|source| scan_one(env, source)).collect();
    // A stable sort keeps the order of `Source::ALL` among sources changed at the same time.
    found.sort_by_key(|found| std::cmp::Reverse(found.modified));
    found
}

/// One source's settings, or `None` when it has no settings file.
pub fn scan_one(env: &Env, source: Source) -> Option<Found> {
    let located = locations::find(source, env);
    if located.is_empty() {
        return None;
    }
    let mut reader =
        Reader::new(if source == Source::Alacritty { alacritty::MAX_DEPTH } else { DEFAULT_DEPTH });
    let mut proposal = match source {
        Source::Ghostty => ghostty::import(env, &mut reader),
        Source::ITerm2 => iterm::import(env, &mut reader),
        Source::TerminalApp => terminal_app::import(env, &mut reader),
        Source::Alacritty => alacritty::import(env, &mut reader),
        Source::Kitty => kitty::import(env, &mut reader),
        Source::WezTerm => wezterm::import(env, &mut reader),
        Source::Warp => warp::import(env, &mut reader),
        Source::Zed => zed::import(env, &mut reader),
        Source::VsCode => vscode::import(env, &mut reader),
    };
    let (mut files, file_problems) = reader.finish();
    // The problems of a file come before the keys of the rest.
    proposal.skipped.splice(0..0, file_problems);
    for path in located {
        if !files.contains(&path) {
            files.push(path);
        }
    }
    let modified =
        files.iter().filter_map(|path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok()).max();
    Some(Found { source, files, modified, proposal })
}
