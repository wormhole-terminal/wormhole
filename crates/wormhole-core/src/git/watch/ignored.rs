//! The paths git ignores in a work tree, so a build's output does not read the repository once a
//! second. Only a way to read less: a path missing from the list is read about (more reads, never a
//! missed change), so every failure and every cap here ends in "less is ignored".

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use anyhow::Result;

use crate::git::GitCmd;

/// The most of git's listing that is read (1 MiB, about 16,000 paths): a repository with more ignored
/// paths than that is listed as having none, and a hostile one cannot make this read without end.
const MAX_LISTING: u64 = 1 << 20;

/// The most entries kept, past which the rest are not ignored.
pub(crate) const MAX_ENTRIES: usize = 8192;

/// Ignored files and folders, relative to the work tree. A folder stands for everything in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ignored {
    entries: HashSet<PathBuf>,
}

impl Ignored {
    /// From the NUL-separated output of
    /// `git ls-files --others --ignored --exclude-standard --directory -z`: ignored folders end in `/`
    /// and are listed without descending into them. Absolute paths and paths with `..` are dropped.
    pub fn parse(raw: &[u8]) -> Self {
        let entries = raw
            .split(|&b| b == 0)
            .filter_map(|entry| {
                let entry = std::str::from_utf8(entry).ok()?;
                let path = Path::new(entry.strip_suffix('/').unwrap_or(entry));
                let plain = path.components().next().is_some()
                    && path.components().all(|part| matches!(part, Component::Normal(_)));
                plain.then(|| path.to_path_buf())
            })
            .take(MAX_ENTRIES)
            .collect();
        Self { entries }
    }

    /// Ask git (read-only) for `work_tree`'s ignored paths. Fails when git does or the listing is over
    /// the cap; the caller then goes on with [`Ignored::default`] (nothing ignored).
    pub fn read(work_tree: &Path) -> Result<Self> {
        let raw = GitCmd::new(work_tree)
            .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
            .max_output(MAX_LISTING)
            .run()?;
        Ok(Self::parse(&raw))
    }

    /// Whether `rel` is an ignored path or inside an ignored folder.
    pub fn covers(&self, rel: &Path) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let mut prefix = PathBuf::new();
        rel.components().any(|part| {
            prefix.push(part);
            self.entries.contains(&prefix)
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_covers_what_is_inside_it() {
        let ignored = Ignored::parse(b"target/\0web/node_modules/\0.env\0");
        assert!(ignored.covers(Path::new("target")));
        assert!(ignored.covers(Path::new("target/debug/deps/a.o")));
        assert!(ignored.covers(Path::new("web/node_modules/x/y.js")));
        assert!(ignored.covers(Path::new(".env")));
        assert!(!ignored.covers(Path::new("web/src/a.js")));
        assert!(!ignored.covers(Path::new("targets/a")), "a name is not a prefix of another");
    }

    #[test]
    fn what_could_name_a_place_outside_is_dropped() {
        let ignored = Ignored::parse(b"/etc/\0../up/\0a/../b/\0\0ok/\0");
        assert_eq!(ignored.len(), 1);
        assert!(ignored.covers(Path::new("ok/x")));
    }

    #[test]
    fn a_listing_longer_than_the_cap_keeps_the_cap() {
        let raw: Vec<u8> = (0..MAX_ENTRIES + 10).flat_map(|n| format!("d{n}/\0").into_bytes()).collect();
        assert_eq!(Ignored::parse(&raw).len(), MAX_ENTRIES);
    }

    #[test]
    fn nothing_listed_covers_nothing() {
        assert!(!Ignored::default().covers(Path::new("a/b")));
    }
}
