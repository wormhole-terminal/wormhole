//! The folders pinned in the Files tab's path menu, kept as absolute paths in `files_pinned`.

use std::path::{Path, PathBuf};

use super::Settings;

/// How many folders can be pinned: the path menu is a menu, not a bookmarks manager.
const MAX_PINNED: usize = 24;

impl Settings {
    /// Whether `folder` is pinned.
    pub fn is_pinned(&self, folder: &Path) -> bool {
        pinned_key(folder).is_some_and(|key| self.files_pinned.contains(&key))
    }

    /// Pins `folder` (at the end of the list), or takes it off again. Returns whether anything
    /// changed: a relative path or one that is not UTF-8 is never pinned (the list is text), and a full
    /// list takes no more.
    pub fn set_pinned(&mut self, folder: &Path, pinned: bool) -> bool {
        let Some(key) = pinned_key(folder).filter(|_| folder.is_absolute()) else { return false };
        let at = self.files_pinned.iter().position(|existing| *existing == key);
        match (pinned, at) {
            (true, None) if self.files_pinned.len() < MAX_PINNED => {
                self.files_pinned.push(key);
                true
            }
            (false, Some(at)) => {
                self.files_pinned.remove(at);
                true
            }
            _ => false,
        }
    }

    /// A folder was renamed from `from` to `to`: its pin, and the pins of folders below it, name the new
    /// place, each keeping its place in the list. Returns whether any did.
    pub fn move_pinned(&mut self, from: &Path, to: &Path) -> bool {
        let mut moved = false;
        for key in &mut self.files_pinned {
            let Ok(rest) = Path::new(key.as_str()).strip_prefix(from) else { continue };
            if let Some(new) = pinned_key(&to.join(rest)) {
                *key = new;
                moved = true;
            }
        }
        moved
    }

    /// The pinned folders as paths, in the order they were pinned.
    pub fn pinned_folders(&self) -> Vec<PathBuf> {
        self.files_pinned.iter().map(PathBuf::from).collect()
    }

    /// Reading a file: only absolute paths can name a folder, each once, and no more than
    /// [`MAX_PINNED`].
    pub(super) fn sanitize_pinned(&mut self) {
        let listed = std::mem::take(&mut self.files_pinned);
        for folder in listed {
            let Some(key) = pinned_key(Path::new(&folder)) else { continue };
            if Path::new(&key).is_absolute()
                && !self.files_pinned.contains(&key)
                && self.files_pinned.len() < MAX_PINNED
            {
                self.files_pinned.push(key);
            }
        }
    }
}

/// How a folder is written in `files_pinned`: without a trailing slash or `.` parts, so the folder
/// picked in a panel and the one a terminal reports are the same entry. `None` for a path that is not
/// UTF-8: it would not come back as the same folder.
fn pinned_key(folder: &Path) -> Option<String> {
    folder.components().collect::<PathBuf>().to_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{MAX_PINNED, Settings};

    #[test]
    fn a_folder_is_pinned_once_and_unpinned_by_the_same_path() {
        let mut settings = Settings::default();
        assert!(settings.set_pinned(Path::new("/w/a"), true));
        assert!(!settings.set_pinned(Path::new("/w/a/"), true), "a trailing slash is the same folder");
        assert!(settings.is_pinned(Path::new("/w/a")));
        assert!(!settings.set_pinned(Path::new("/w/b"), false), "nothing to unpin");
        assert!(settings.set_pinned(Path::new("/w/a/"), false));
        assert!(settings.files_pinned.is_empty());
    }

    #[test]
    fn a_renamed_folder_keeps_its_pin_and_the_pins_below_it_in_place() {
        let mut settings = Settings::default();
        for folder in ["/w/a", "/w/src", "/w/src/deep", "/w/srcx", "/w/z"] {
            settings.set_pinned(Path::new(folder), true);
        }
        assert!(settings.move_pinned(Path::new("/w/src"), Path::new("/w/code")));
        assert_eq!(settings.files_pinned, ["/w/a", "/w/code", "/w/code/deep", "/w/srcx", "/w/z"]);
        assert!(!settings.move_pinned(Path::new("/w/none"), Path::new("/w/x")));
    }

    #[test]
    fn a_relative_path_and_a_full_list_are_refused() {
        let mut settings = Settings::default();
        assert!(!settings.set_pinned(Path::new("rel"), true));
        for n in 0..MAX_PINNED {
            assert!(settings.set_pinned(Path::new(&format!("/w/{n}")), true));
        }
        assert!(!settings.set_pinned(Path::new("/w/extra"), true));
    }

    #[test]
    fn a_path_that_is_not_utf8_is_not_pinned() {
        let odd = crate::platform::fs::path_from_bytes(b"/w/bad-\xff-name");
        let mut settings = Settings::default();
        assert!(!settings.set_pinned(odd, true));
        assert!(!settings.is_pinned(odd));
        assert!(settings.files_pinned.is_empty());
    }

    #[test]
    fn reading_keeps_the_absolute_paths_once_each_in_order() {
        let mut settings = Settings {
            files_pinned: ["/b", "rel", "/a/", "/b/", "~/c"].map(String::from).to_vec(),
            ..Settings::default()
        };
        settings.sanitize_pinned();
        assert_eq!(settings.files_pinned, ["/b", "/a"]);
    }
}
