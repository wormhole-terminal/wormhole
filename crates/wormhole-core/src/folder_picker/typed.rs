//! What the text typed in the picker's field means: where the path starts, which steps it has, and how it was
//! written, because a path comes out the way it went in.

use std::path::{Component, Path, PathBuf};

/// The typed text read as a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typed {
    /// How it starts as typed: nothing (relative to the terminal's folder), `/` or `~/`.
    pub anchor: &'static str,
    /// The folder the steps are taken from: the terminal's folder, `/` or home.
    pub start: PathBuf,
    /// The text between the slashes after the anchor. The last is what is being typed (it may be empty, when
    /// the text ends in a slash); the ones before it are folders to go through.
    pub steps: Vec<String>,
}

impl Typed {
    /// The folder steps, without the one being typed.
    pub fn through(&self) -> &[String] {
        &self.steps[..self.steps.len() - 1]
    }

    /// The step being typed.
    pub fn last(&self) -> &str {
        &self.steps[self.steps.len() - 1]
    }
}

/// Read `typed`: with no `/` in it all of it is the step being typed in `cwd`; `src/` goes through `cwd`'s `src`,
/// `../` the folder above, `~/` starts at home, `/usr/` at the root. Only `~/` means home: another user's
/// `~name/` is not looked up.
pub fn parse(typed: &str, cwd: &Path, home: &Path) -> Typed {
    let (anchor, start, rest) = if let Some(rest) = typed.strip_prefix('/') {
        ("/", PathBuf::from("/"), rest)
    } else if let Some(rest) = typed.strip_prefix("~/") {
        ("~/", home.to_path_buf(), rest)
    } else {
        ("", cwd.to_path_buf(), typed)
    };
    Typed { anchor, start, steps: rest.split('/').map(str::to_string).collect() }
}

/// The text one level up from `typed`'s folder part (everything before its last `/`), in the form it is
/// written: `src/ui/bu` goes to `src/`, `src/` to nothing, `/usr/` to `/`, `~/Projects/` to `~/`, the terminal's
/// folder (nothing typed) to `../`, and a path that already ends in `..` goes up once more. `~/` goes to
/// home's parent, written in full; `/` has nothing above.
pub fn up(typed: &str, cwd: &Path, home: &Path) -> Option<String> {
    let folder = &typed[..typed.rfind('/').map_or(0, |at| at + 1)];
    match folder {
        "" => cwd.parent().map(|_| "../".to_string()),
        "/" => None,
        "~/" => home.parent().map(|parent| format!("{}/", parent.display()).replace("//", "/")),
        _ => {
            let parts: Vec<&str> = folder.trim_end_matches('/').split('/').collect();
            if parts.last() == Some(&"..") {
                return Some(format!("{folder}../"));
            }
            let kept = &parts[..parts.len() - 1];
            Some(if kept.is_empty() { String::new() } else { format!("{}/", kept.join("/")) })
        }
    }
}

/// `path` without its `.` and `..` steps, worked out from the text alone (a link is not followed): enough for
/// naming a folder and for telling that two spellings are one.
pub fn tidy(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{parse, tidy, up};

    const CWD: &str = "/work/app";
    const HOME: &str = "/Users/me";

    fn at(typed: &str) -> (&'static str, PathBuf, Vec<String>) {
        let t = parse(typed, Path::new(CWD), Path::new(HOME));
        (t.anchor, t.start, t.steps)
    }

    fn steps(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn nothing_typed_is_an_empty_step_in_the_current_folder() {
        assert_eq!(at(""), ("", PathBuf::from(CWD), steps(&[""])));
    }

    #[test]
    fn text_without_a_slash_is_the_step_being_typed() {
        assert_eq!(at("sr"), ("", PathBuf::from(CWD), steps(&["sr"])));
    }

    #[test]
    fn a_relative_path_has_a_step_for_each_folder() {
        assert_eq!(at("src/"), ("", PathBuf::from(CWD), steps(&["src", ""])));
        assert_eq!(at("src/ui/bu"), ("", PathBuf::from(CWD), steps(&["src", "ui", "bu"])));
        assert_eq!(at("pr/te"), ("", PathBuf::from(CWD), steps(&["pr", "te"])));
    }

    #[test]
    fn dots_are_steps_like_any_other() {
        assert_eq!(at("../../x"), ("", PathBuf::from(CWD), steps(&["..", "..", "x"])));
        assert_eq!(at("./"), ("", PathBuf::from(CWD), steps(&[".", ""])));
    }

    #[test]
    fn a_tilde_slash_starts_at_home_and_an_absolute_path_at_the_root() {
        assert_eq!(at("~/"), ("~/", PathBuf::from(HOME), steps(&[""])));
        assert_eq!(at("~/Projects/te"), ("~/", PathBuf::from(HOME), steps(&["Projects", "te"])));
        assert_eq!(at("/usr/lo"), ("/", PathBuf::from("/"), steps(&["usr", "lo"])));
        assert_eq!(at("/"), ("/", PathBuf::from("/"), steps(&[""])));
    }

    #[test]
    fn a_tilde_alone_or_with_a_name_is_not_home() {
        assert_eq!(at("~"), ("", PathBuf::from(CWD), steps(&["~"])));
        assert_eq!(at("~bob/x"), ("", PathBuf::from(CWD), steps(&["~bob", "x"])));
    }

    #[test]
    fn spaces_and_dots_in_names_are_kept() {
        assert_eq!(at("My Folder/.gi"), ("", PathBuf::from(CWD), steps(&["My Folder", ".gi"])));
    }

    #[test]
    fn up_is_the_text_one_level_above_the_folder_typed() {
        let (cwd, home) = (Path::new(CWD), Path::new(HOME));
        let go = |t: &str| up(t, cwd, home);
        assert_eq!(go(""), Some("../".into()));
        assert_eq!(go("sr"), Some("../".into()), "the filter is not a level");
        assert_eq!(go("src/"), Some(String::new()));
        assert_eq!(go("src/ui/bu"), Some("src/".into()));
        assert_eq!(go("../"), Some("../../".into()));
        assert_eq!(go("src/../"), Some("src/../../".into()));
        assert_eq!(go("/usr/"), Some("/".into()));
        assert_eq!(go("/usr/local/"), Some("/usr/".into()));
        assert_eq!(go("/"), None);
        assert_eq!(go("~/Projects/"), Some("~/".into()));
        assert_eq!(go("~/"), Some("/Users/".into()));
        assert_eq!(up("", Path::new("/"), home), None, "nothing above the root");
    }

    #[test]
    fn tidy_resolves_dots_by_the_text_alone() {
        assert_eq!(tidy(Path::new("/a/b/../c/./d")), PathBuf::from("/a/c/d"));
        assert_eq!(tidy(Path::new("/a/..")), PathBuf::from("/"));
        assert_eq!(tidy(Path::new("a/../..")), PathBuf::from(".."));
    }
}
