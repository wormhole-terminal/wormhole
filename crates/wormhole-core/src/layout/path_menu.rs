//! The folders a path menu lists: the title row's folder clicked opens a menu like the one a
//! Finder window's title does, the folder first and then each parent down to the volume.
//! This is only that list; the icons are the platform's and the items are the app's.

use std::path::{Path, PathBuf};

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathFolder {
    pub path: PathBuf,
    /// What the line says: the folder's own name, or the volume's name for the top of the tree.
    pub name: String,
}

/// `folder` and its parents, innermost first, down to the volume. The volume is `/` (named
/// `boot_volume`, "Macintosh HD") or, for a path under `/Volumes/<name>`, that volume's folder,
/// where the list ends as Finder's does: the folders above it are not the person's to wander.
/// Nothing is read from disk: the names are the path's own.
pub fn chain(folder: &Path, boot_volume: &str) -> Vec<PathFolder> {
    let mut parts = folder.components();
    let volume_root = match (parts.next(), parts.next(), parts.next()) {
        (Some(root), Some(volumes), Some(name))
            if matches!(root, std::path::Component::RootDir)
                && volumes.as_os_str() == "Volumes"
                && matches!(name, std::path::Component::Normal(_)) =>
        {
            Some(Path::new("/Volumes").join(name.as_os_str()))
        }
        _ => None,
    };
    let mut out = Vec::new();
    for ancestor in folder.ancestors() {
        let name = match ancestor.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            // The root; a relative path ends in an empty one, which is nothing.
            None if ancestor.has_root() => boot_volume.to_string(),
            None => break,
        };
        out.push(PathFolder { path: ancestor.to_path_buf(), name });
        if volume_root.as_deref() == Some(ancestor) {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::chain;

    fn names(path: &str) -> Vec<String> {
        chain(Path::new(path), "Macintosh HD").into_iter().map(|folder| folder.name).collect()
    }

    #[test]
    fn a_path_under_the_home_folder_goes_down_to_the_boot_volume() {
        assert_eq!(names("/Users/ben/Projects/tube"), ["tube", "Projects", "ben", "Users", "Macintosh HD"]);
        let folders = chain(Path::new("/Users/ben/Projects"), "Macintosh HD");
        assert_eq!(folders[0].path, Path::new("/Users/ben/Projects"));
        assert_eq!(folders[1].path, Path::new("/Users/ben"));
        assert_eq!(folders[3].path, Path::new("/"));
    }

    #[test]
    fn a_path_outside_home_is_just_as_deep() {
        assert_eq!(names("/tmp"), ["tmp", "Macintosh HD"]);
    }

    #[test]
    fn the_volume_root_is_the_volume_alone() {
        assert_eq!(names("/"), ["Macintosh HD"]);
    }

    #[test]
    fn another_volume_ends_the_list_at_its_own_name() {
        assert_eq!(names("/Volumes/Work/clients/acme"), ["acme", "clients", "Work"]);
        assert_eq!(names("/Volumes/Work"), ["Work"]);
        // Not a volume yet: the folder that holds them is an ordinary one.
        assert_eq!(names("/Volumes"), ["Volumes", "Macintosh HD"]);
    }

    #[test]
    fn a_relative_path_has_no_volume() {
        assert_eq!(names("a/b"), ["b", "a"]);
        assert!(names("").is_empty());
    }
}
