//! Grouping the terminal list into folder bands: the grouping modes and the bucketing behind
//! them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How the sidebar groups its sessions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grouping {
    /// One flat list, in the order the user gave them.
    #[default]
    None,
    /// One band per folder: the repository a session is in, else its working directory.
    Folder,
}

impl Grouping {
    pub const ALL: [Grouping; 2] = [Grouping::None, Grouping::Folder];

    pub fn label(self) -> &'static str {
        match self {
            Grouping::None => "No grouping",
            Grouping::Folder => "By folder",
        }
    }

    /// Session indices in the order the sidebar lists them (before pins, folding and the
    /// filter: see `Listing`).
    pub fn order(self, folders: &[Option<PathBuf>], pinned_folders: &[PathBuf]) -> Vec<usize> {
        match self {
            Grouping::None => (0..folders.len()).collect(),
            Grouping::Folder => {
                bucket(folders, pinned_folders).into_iter().flat_map(|bucket| bucket.sessions).collect()
            }
        }
    }
}

/// The sessions of one folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// `None` collects sessions whose folder isn't known yet.
    pub folder: Option<PathBuf>,
    /// Header text; unique among the groups of one call.
    pub label: String,
    /// Indices into the input, in their original order. Empty for a pinned folder nobody is in.
    pub sessions: Vec<usize>,
    /// The folder is one the user pinned: its band stays in the list without any session.
    pub pinned: bool,
}

/// Bucket sessions by folder. The pinned folders come first, in the order they were pinned,
/// each present even with no session in it; then the others in the order their first session
/// appears, so a new session never reshuffles the sections above it; sessions with no known
/// folder come last.
pub fn group_by_folder(folders: &[Option<PathBuf>], pinned: &[PathBuf], home: Option<&Path>) -> Vec<Group> {
    let buckets = bucket(folders, pinned);
    let names: Vec<String> =
        buckets.iter().map(|bucket| folder_name(bucket.folder.as_deref(), home)).collect();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for name in &names {
        *seen.entry(name).or_default() += 1;
    }
    buckets
        .into_iter()
        .zip(&names)
        .map(|(Bucket { folder, sessions, pinned }, name)| {
            // `app` twice would be ambiguous: spell out where each one lives.
            let label = match &folder {
                Some(folder) if seen[name.as_str()] > 1 => display_path(folder, home),
                _ => name.clone(),
            };
            Group { folder, label, sessions, pinned }
        })
        .collect()
}

struct Bucket {
    folder: Option<PathBuf>,
    sessions: Vec<usize>,
    pinned: bool,
}

fn bucket(folders: &[Option<PathBuf>], pinned: &[PathBuf]) -> Vec<Bucket> {
    let mut buckets: Vec<Bucket> = Vec::new();
    for folder in pinned {
        // Pinned twice (a hand-edited list) is still one band.
        if !buckets.iter().any(|known| known.folder.as_ref() == Some(folder)) {
            buckets.push(Bucket { folder: Some(folder.clone()), sessions: Vec::new(), pinned: true });
        }
    }
    for (ix, folder) in folders.iter().enumerate() {
        match buckets.iter_mut().find(|known| &known.folder == folder) {
            Some(known) => known.sessions.push(ix),
            None => buckets.push(Bucket { folder: folder.clone(), sessions: vec![ix], pinned: false }),
        }
    }
    // Stable, so the pinned folders keep their pin order and the others their first-seen order.
    buckets.sort_by_key(|bucket| bucket.folder.is_none());
    buckets
}

/// The label of the band of sessions whose folder isn't known yet.
pub const NO_FOLDER: &str = "No folder";

fn folder_name(folder: Option<&Path>, home: Option<&Path>) -> String {
    let Some(folder) = folder else { return NO_FOLDER.into() };
    if Some(folder) == home {
        return "~".into();
    }
    match folder.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        // The filesystem root has no name.
        None => folder.display().to_string(),
    }
}

/// Where `cwd` is inside `folder`, e.g. `crates/core`; `None` at the folder itself
/// or outside it. In the grouped list the header already names the folder, so a
/// session's row only needs to say when it has moved deeper in.
pub fn place_in_folder(cwd: &Path, folder: &Path) -> Option<String> {
    let rest = cwd.strip_prefix(folder).ok()?;
    (!rest.as_os_str().is_empty()).then(|| rest.display().to_string())
}

/// `~/Projects/app` for paths under the home directory, the full path otherwise.
pub fn display_path(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Grouping, display_path, group_by_folder, place_in_folder};
    use std::path::{Path, PathBuf};

    fn dirs(paths: &[Option<&str>]) -> Vec<Option<PathBuf>> {
        paths.iter().map(|p| p.map(PathBuf::from)).collect()
    }

    #[test]
    fn groups_keep_first_seen_order_and_session_order() {
        let folders = dirs(&[Some("/w/b"), Some("/w/a"), Some("/w/b"), Some("/w/a"), Some("/w/b")]);
        let groups = group_by_folder(&folders, &[], None);
        let shape: Vec<_> = groups.iter().map(|g| (g.label.as_str(), g.sessions.clone())).collect();
        assert_eq!(shape, vec![("b", vec![0, 2, 4]), ("a", vec![1, 3])]);
    }

    #[test]
    fn unknown_folders_come_last() {
        let folders = dirs(&[None, Some("/w/a"), None, Some("/w/b")]);
        let groups = group_by_folder(&folders, &[], None);
        let shape: Vec<_> = groups.iter().map(|g| (g.label.as_str(), g.sessions.clone())).collect();
        assert_eq!(shape, vec![("a", vec![1]), ("b", vec![3]), ("No folder", vec![0, 2])]);
    }

    #[test]
    fn home_is_tilde() {
        let home = Path::new("/Users/me");
        let groups = group_by_folder(&dirs(&[Some("/Users/me")]), &[], Some(home));
        assert_eq!(groups[0].label, "~");
    }

    #[test]
    fn same_folder_name_is_disambiguated_by_path() {
        let home = Path::new("/Users/me");
        let folders = dirs(&[Some("/Users/me/a/app"), Some("/Users/me/b/app"), Some("/Users/me/c")]);
        let labels: Vec<_> =
            group_by_folder(&folders, &[], Some(home)).into_iter().map(|g| g.label).collect();
        assert_eq!(labels, vec!["~/a/app", "~/b/app", "c"]);
    }

    #[test]
    fn filesystem_root_keeps_a_label() {
        let groups = group_by_folder(&dirs(&[Some("/")]), &[], None);
        assert_eq!(groups[0].label, "/");
    }

    #[test]
    fn order_follows_the_sections() {
        let folders = dirs(&[Some("/w/b"), Some("/w/a"), None, Some("/w/b")]);
        assert_eq!(Grouping::None.order(&folders, &[]), vec![0, 1, 2, 3]);
        assert_eq!(Grouping::Folder.order(&folders, &[]), vec![0, 3, 1, 2]);
    }

    #[test]
    fn empty_list_has_no_groups() {
        assert!(group_by_folder(&[], &[], None).is_empty());
        assert!(Grouping::Folder.order(&[], &[]).is_empty());
    }

    fn paths(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn pinned_folders_lead_in_pin_order_even_without_sessions() {
        let folders = dirs(&[Some("/w/b"), None, Some("/w/a")]);
        let groups = group_by_folder(&folders, &paths(&["/w/z", "/w/a"]), None);
        let shape: Vec<_> = groups.iter().map(|g| (g.label.as_str(), g.sessions.clone(), g.pinned)).collect();
        assert_eq!(
            shape,
            vec![
                ("z", vec![], true),
                ("a", vec![2], true),
                ("b", vec![0], false),
                ("No folder", vec![1], false)
            ]
        );
        // Only the folders' places change: the sessions are still the ones of their folder.
        assert_eq!(Grouping::Folder.order(&folders, &paths(&["/w/a"])), vec![2, 0, 1]);
        assert_eq!(Grouping::None.order(&folders, &paths(&["/w/a"])), vec![0, 1, 2]);
    }

    #[test]
    fn a_folder_pinned_twice_is_one_band() {
        let groups = group_by_folder(&[], &paths(&["/w/a", "/w/b", "/w/a/"]), None);
        assert_eq!(groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn an_empty_pinned_folder_counts_when_two_names_are_the_same() {
        let home = Path::new("/Users/me");
        let folders = dirs(&[Some("/Users/me/b/app"), Some("/Users/me/c")]);
        let groups = group_by_folder(&folders, &paths(&["/Users/me/a/app"]), Some(home));
        let labels: Vec<_> = groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["~/a/app", "~/b/app", "c"]);
    }

    #[test]
    fn display_path_abbreviates_home() {
        let home = Path::new("/Users/me");
        assert_eq!(display_path(Path::new("/Users/me/Projects/x"), Some(home)), "~/Projects/x");
        assert_eq!(display_path(Path::new("/opt/x"), Some(home)), "/opt/x");
        assert_eq!(display_path(Path::new("/Users/me"), Some(home)), "~");
    }

    #[test]
    fn place_in_folder_is_the_path_below_it() {
        let folder = Path::new("/w/app");
        assert_eq!(place_in_folder(Path::new("/w/app/crates/core"), folder).as_deref(), Some("crates/core"));
        assert_eq!(place_in_folder(Path::new("/w/app"), folder), None);
        assert_eq!(place_in_folder(Path::new("/w/other"), folder), None);
        assert_eq!(place_in_folder(Path::new("/w/application"), folder), None);
    }

    #[test]
    fn grouping_serializes_as_a_lowercase_word() {
        assert_eq!(serde_json::to_string(&Grouping::Folder).unwrap_or_default(), "\"folder\"");
        assert_eq!(serde_json::from_str::<Grouping>("\"none\"").ok(), Some(Grouping::None));
    }
}
