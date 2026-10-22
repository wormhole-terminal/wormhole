//! Wormhole as the Mac's default terminal: the kinds of file it takes over from Terminal, and what
//! a terminal does with a file or folder macOS asks it to open (a double-clicked `.command`, Open
//! With in Finder, a folder dropped on the Dock icon).
//!
//! The person opened the file themselves, as when they double-click it with Terminal as the
//! default, so running a script here needs no project trust: it is what they asked for.

use std::path::{Path, PathBuf};

use crate::terminal::shells::single_quote;
use wormhole_term::percent_decode;

/// The content types the default terminal is the shell for: Terminal's own scripts (`.command`,
/// `.tool`) and a program without an extension. The plain `.sh` (`public.shell-script`) is left to
/// the editor it opens in now: double-clicking one edits it, on any Mac.
pub const SCRIPT_TYPES: &[&str] = &["com.apple.terminal.shell-script", "public.unix-executable"];

/// What a new terminal does with something macOS asked Wormhole to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// A folder: a terminal in it.
    Folder(PathBuf),
    /// A script or program: a terminal in its folder, with this line typed into the shell and run.
    Script { folder: PathBuf, line: String },
}

impl Opened {
    /// What to do with the item at `path`, which is a folder when `is_dir`.
    pub fn of(path: &Path, is_dir: bool) -> Option<Self> {
        if is_dir {
            return Some(Self::Folder(path.to_path_buf()));
        }
        let folder = path.parent()?.to_path_buf();
        // Quoted always: a script's name may hold a space, a quote or a `;`.
        Some(Self::Script { folder, line: single_quote(path.to_str()?) })
    }

    /// The folder the terminal starts in.
    pub fn folder(&self) -> &Path {
        match self {
            Self::Folder(folder) | Self::Script { folder, .. } => folder,
        }
    }

    /// The line typed into the new terminal's shell, if any.
    pub fn line(&self) -> Option<&str> {
        match self {
            Self::Folder(_) => None,
            Self::Script { line, .. } => Some(line),
        }
    }
}

/// The local path a `file://` URL names (`file:///a%20b/` → `/a b`); `None` for any other URL, a
/// file on another host, or a path that isn't absolute or isn't text.
pub fn file_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    let (host, path) = rest.split_at(slash);
    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        return None;
    }
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let path = percent_decode(path.as_bytes())?;
    if path.contains('\0') {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    Some(PathBuf::from(if trimmed.is_empty() { "/" } else { trimmed }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_become_paths() {
        assert_eq!(file_path("file:///Users/me/run.command"), Some(PathBuf::from("/Users/me/run.command")));
        assert_eq!(file_path("file:///Users/me/My%20Scripts/"), Some(PathBuf::from("/Users/me/My Scripts")));
        assert_eq!(file_path("file://localhost/tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(file_path("file:///"), Some(PathBuf::from("/")));
        assert_eq!(file_path("file:///caf%C3%A9"), Some(PathBuf::from("/café")));
    }

    #[test]
    fn other_urls_are_refused() {
        assert_eq!(file_path("ssh://host"), None);
        assert_eq!(file_path("https://example.com/x"), None);
        assert_eq!(file_path("file://server/share/x"), None);
        assert_eq!(file_path("file:///a%00b"), None);
        assert_eq!(file_path("file:///bad%ff"), None);
    }

    #[test]
    fn a_folder_opens_a_terminal_in_it() {
        let opened = Opened::of(Path::new("/Users/me/repo"), true);
        assert_eq!(opened, Some(Opened::Folder(PathBuf::from("/Users/me/repo"))));
        assert_eq!(opened.as_ref().and_then(Opened::line), None);
    }

    #[test]
    fn a_script_runs_quoted_in_its_folder() {
        let opened = Opened::of(Path::new("/Users/me/it's here; rm.command"), false);
        let Some(opened) = opened else { panic!("a script opens") };
        assert_eq!(opened.folder(), Path::new("/Users/me"));
        assert_eq!(opened.line(), Some(r"'/Users/me/it'\''s here; rm.command'"));
    }
}
