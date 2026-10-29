//! What the Studio is editing and where it lives: the person's own theme file, a copy of another
//! theme that has no file yet, or the trusted project's theme. Pure: which one a Studio opens on
//! is decided here from what the settings and the project say, and tested without a window.

use std::path::PathBuf;

use crate::studio::themes::Found;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::studio) enum Subject {
    /// A file in the profile's themes folder, edited in place: saving writes it and keeps its file name.
    Own { path: PathBuf },
    /// A copy of another theme with no file yet; the first save asks for a name. `from` is the
    /// name of the theme it started from.
    Untitled { from: String },
    /// `.wormhole/theme.json` of a trusted project, written in the older format.
    Project { root: PathBuf },
}

impl Subject {
    /// The word devctl's `state` line uses.
    #[cfg(feature = "devctl")]
    pub(in crate::studio) fn word(&self) -> String {
        match self {
            Self::Own { path } => {
                format!("own:{}", path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default())
            }
            Self::Untitled { from } => format!("untitled:{}", from.replace(' ', "_")),
            Self::Project { .. } => "project".to_string(),
        }
    }
}

/// The subject to open on. `project` is the root of the trusted project whose theme is in effect;
/// `shown` is the pick the window shows now (the dark or the light one by the system's
/// appearance), `locate` what it selects and `fallback` the name of the theme in use, for a pick
/// that is not a file of its own.
pub(in crate::studio) fn subject_for(
    project: Option<PathBuf>,
    shown: &str,
    locate: &dyn Fn(&str) -> Found,
    fallback: &str,
) -> Subject {
    if let Some(root) = project {
        return Subject::Project { root };
    }
    match locate(shown) {
        Found::File(path) | Found::Variant(path, _) => Subject::Own { path },
        Found::Builtin(name) => Subject::Untitled { from: name },
        Found::Missing => Subject::Untitled { from: fallback.to_string() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(project: Option<&str>, shown: &str) -> Subject {
        let locate = |setting: &str| match setting {
            "mine" => Found::File("/themes/mine.json".into()),
            "Nord" => Found::Builtin("Nord".into()),
            "pair" => Found::Variant("/themes/pair.json".into(), "dark".into()),
            _ => Found::Missing,
        };
        subject_for(project.map(PathBuf::from), shown, &locate, "Wormhole Dark")
    }

    #[test]
    fn the_studio_opens_on_the_theme_in_use() {
        assert_eq!(open(None, "mine"), Subject::Own { path: "/themes/mine.json".into() });
        assert_eq!(open(None, "Nord"), Subject::Untitled { from: "Nord".into() });
        assert_eq!(
            open(None, "pair"),
            Subject::Own { path: "/themes/pair.json".into() },
            "a pair is edited as one file"
        );
        assert_eq!(open(None, "gone"), Subject::Untitled { from: "Wormhole Dark".into() });
    }

    #[test]
    fn a_trusted_project_theme_in_effect_comes_first() {
        assert_eq!(open(Some("/work/app"), "mine"), Subject::Project { root: "/work/app".into() });
    }
}
