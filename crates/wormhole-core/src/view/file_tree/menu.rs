//! What the Files tab's path menu lists, like the menu a Finder window's title opens: the folder the tab
//! shows and each folder above it, the pinned folders, and the commands for pinning and for going back to
//! the terminal's folder. Only the list: the app words it, draws it and acts on a choice.

use std::path::{Path, PathBuf};

use super::names::clean;
use crate::view::layout::path_menu::{PathFolder, chain};
use crate::view::sidebar::display_path;

/// Pin the shown folder, or take it off the pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pin {
    /// "Pin “name”".
    Pin(String),
    /// "Unpin “name”".
    Unpin(String),
}

/// A pinned folder as a menu line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedFolder {
    pub path: PathBuf,
    /// Its name; with the folder it is in as well when another pin has the same name.
    pub title: String,
}

/// The path menu for a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathMenu {
    /// The shown folder first (it is the one with the check), then each parent down to the volume.
    pub folders: Vec<PathFolder>,
    pub pinned: Vec<PinnedFolder>,
    pub pin: Pin,
    /// A folder was chosen by hand, so "Follow Terminal" has something to undo.
    pub can_follow: bool,
}

impl PathMenu {
    /// The menu of `shown`. `volume` names `/` (the disk), `pinned` are the pins in the order they were
    /// made, `chosen` says the folder was picked by hand and `home` is only used to tell two pinned
    /// folders of the same name apart.
    pub fn of(shown: &Path, volume: &str, pinned: &[PathBuf], chosen: bool, home: Option<&Path>) -> Self {
        // A folder's name is whatever its maker chose: cleaned like a row's.
        let folders: Vec<PathFolder> = chain(shown, volume)
            .into_iter()
            .map(|folder| PathFolder { name: clean(&folder.name), ..folder })
            .collect();
        let name = folders.first().map_or_else(|| clean(&shown.display().to_string()), |f| f.name.clone());
        let pin = if pinned.iter().any(|path| path == shown) { Pin::Unpin(name) } else { Pin::Pin(name) };
        Self { folders, pinned: pinned_lines(pinned, volume, home), pin, can_follow: chosen }
    }
}

fn pinned_lines(pinned: &[PathBuf], volume: &str, home: Option<&Path>) -> Vec<PinnedFolder> {
    let name_of = |path: &Path| match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => volume.to_string(),
    };
    pinned
        .iter()
        .map(|path| {
            let name = clean(&name_of(path));
            let twin = pinned.iter().any(|other| other != path && clean(&name_of(other)) == name);
            let parent = path.parent().filter(|parent| twin && !parent.as_os_str().is_empty());
            let title = match parent {
                Some(parent) => format!("{name} — {}", clean(&display_path(parent, home))),
                None => name,
            };
            PinnedFolder { path: path.clone(), title }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{PathMenu, Pin};

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_shown_folder_leads_and_its_parents_follow_down_to_the_disk() {
        let menu = PathMenu::of(Path::new("/Users/ben/code/app"), "Macintosh HD", &[], false, None);
        let names: Vec<&str> = menu.folders.iter().map(|folder| folder.name.as_str()).collect();
        assert_eq!(names, ["app", "code", "ben", "Users", "Macintosh HD"]);
        assert_eq!(menu.folders[0].path, Path::new("/Users/ben/code/app"));
        assert_eq!(menu.pin, Pin::Pin("app".into()));
        assert!(menu.pinned.is_empty() && !menu.can_follow);
    }

    #[test]
    fn a_pinned_folder_offers_to_unpin_and_a_chosen_one_to_follow_the_terminal_again() {
        let pins = paths(&["/Users/ben/code/app", "/tmp"]);
        let menu = PathMenu::of(Path::new("/Users/ben/code/app"), "Macintosh HD", &pins, true, None);
        assert_eq!(menu.pin, Pin::Unpin("app".into()));
        assert!(menu.can_follow);
        let titles: Vec<&str> = menu.pinned.iter().map(|pin| pin.title.as_str()).collect();
        assert_eq!(titles, ["app", "tmp"]);
    }

    #[test]
    fn two_pins_with_one_name_say_where_each_is() {
        let pins = paths(&["/Users/ben/a/app", "/Users/ben/b/app", "/srv/www"]);
        let menu = PathMenu::of(Path::new("/"), "Macintosh HD", &pins, false, Some(Path::new("/Users/ben")));
        let titles: Vec<&str> = menu.pinned.iter().map(|pin| pin.title.as_str()).collect();
        assert_eq!(titles, ["app — ~/a", "app — ~/b", "www"]);
        assert_eq!(menu.folders.len(), 1, "the disk alone");
        assert_eq!(menu.pin, Pin::Pin("Macintosh HD".into()));
    }

    #[test]
    fn names_are_cleaned_like_a_rows() {
        let menu =
            PathMenu::of(Path::new("/tmp/a\nb"), "Macintosh HD", &paths(&["/x/c\u{202e}d"]), false, None);
        assert_eq!(menu.folders[0].name, "a\u{FFFD}b");
        assert_eq!(menu.pin, Pin::Pin("a\u{FFFD}b".into()));
        assert_eq!(menu.pinned[0].title, "c\u{FFFD}d");
    }

    #[test]
    fn a_pinned_disk_is_named_as_the_disk() {
        let menu = PathMenu::of(Path::new("/tmp"), "Macintosh HD", &paths(&["/"]), false, None);
        assert_eq!(menu.pinned[0].title, "Macintosh HD");
    }
}
