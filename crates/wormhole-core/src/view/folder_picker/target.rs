//! What the text typed in the picker's field means: which folder to list, what narrows it, and how the
//! path was written, because a path comes out the way it went in.

use std::path::{Path, PathBuf};

/// The typed text resolved against the terminal's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The folder whose subfolders are listed. Not made tidy: a `..` in it is the file system's to resolve,
    /// as it is for any command given the same text.
    pub folder: PathBuf,
    /// The text after the last `/`: it narrows the list.
    pub filter: String,
    /// The text up to and including the last `/`, as typed (`src/`, `../`, `~/`, `/usr/`): the path of a
    /// picked folder starts with it, so it stays relative, `~` or absolute as it was written.
    pub prefix: String,
}

/// Resolve `typed`: with no `/` in it the folder is `cwd` and all of it narrows the list; `src/` is `cwd`'s
/// `src`, `../` the folder above `cwd`, `~/` the home folder, `/usr/` that folder. Only `~/` means home:
/// another user's `~name/` is not looked up.
pub fn resolve(typed: &str, cwd: &Path, home: &Path) -> Target {
    let (prefix, filter) = match typed.rfind('/') {
        Some(slash) => typed.split_at(slash + 1),
        None => ("", typed),
    };
    let folder = if prefix.is_empty() {
        cwd.to_path_buf()
    } else if prefix.starts_with('/') {
        PathBuf::from(prefix)
    } else if let Some(below_home) = prefix.strip_prefix("~/") {
        home.join(below_home)
    } else {
        cwd.join(prefix)
    };
    Target { folder, filter: filter.to_string(), prefix: prefix.to_string() }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::resolve;

    const CWD: &str = "/work/app";
    const HOME: &str = "/Users/me";

    fn at(typed: &str) -> (PathBuf, String, String) {
        let t = resolve(typed, Path::new(CWD), Path::new(HOME));
        (t.folder, t.filter, t.prefix)
    }

    #[test]
    fn nothing_typed_lists_the_current_folder() {
        assert_eq!(at(""), (PathBuf::from(CWD), String::new(), String::new()));
    }

    #[test]
    fn text_without_a_slash_narrows_the_current_folder() {
        assert_eq!(at("sr"), (PathBuf::from(CWD), "sr".into(), String::new()));
    }

    #[test]
    fn a_relative_path_walks_down() {
        assert_eq!(at("src/"), (PathBuf::from("/work/app/src/"), String::new(), "src/".into()));
        assert_eq!(at("src/ui/bu"), (PathBuf::from("/work/app/src/ui/"), "bu".into(), "src/ui/".into()));
    }

    #[test]
    fn dots_walk_up_and_stay_as_typed() {
        assert_eq!(at("../"), (PathBuf::from("/work/app/../"), String::new(), "../".into()));
        assert_eq!(at("../../x"), (PathBuf::from("/work/app/../../"), "x".into(), "../../".into()));
        assert_eq!(at("./"), (PathBuf::from("/work/app/./"), String::new(), "./".into()));
    }

    #[test]
    fn a_tilde_slash_is_home_and_stays_a_tilde() {
        assert_eq!(at("~/"), (PathBuf::from("/Users/me/"), String::new(), "~/".into()));
        assert_eq!(at("~/Pro"), (PathBuf::from("/Users/me/"), "Pro".into(), "~/".into()));
        assert_eq!(
            at("~/Projects/te"),
            (PathBuf::from("/Users/me/Projects/"), "te".into(), "~/Projects/".into())
        );
    }

    #[test]
    fn a_tilde_alone_or_with_a_name_is_not_home() {
        assert_eq!(at("~"), (PathBuf::from(CWD), "~".into(), String::new()));
        assert_eq!(at("~bob/x"), (PathBuf::from("/work/app/~bob/"), "x".into(), "~bob/".into()));
    }

    #[test]
    fn an_absolute_path_is_taken_whole() {
        assert_eq!(at("/"), (PathBuf::from("/"), String::new(), "/".into()));
        assert_eq!(at("/usr/lo"), (PathBuf::from("/usr/"), "lo".into(), "/usr/".into()));
    }

    #[test]
    fn spaces_and_dots_in_names_are_kept() {
        assert_eq!(
            at("My Folder/.gi"),
            (PathBuf::from("/work/app/My Folder/"), ".gi".into(), "My Folder/".into())
        );
    }
}
