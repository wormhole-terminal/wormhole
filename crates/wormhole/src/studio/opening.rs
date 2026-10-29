//! What the Studio opens on: the theme in use. The person's own theme file is edited in place; a
//! built-in becomes "Untitled (from X)", a copy with nothing of its own yet; a trusted project's
//! theme is edited as the project has it. Decided from what the settings and the project say
//! (`Inputs`, gathered before the window opens) so the choice is tested without a window.

use std::path::PathBuf;

use wormhole_core::theme::tokens::{ThemeFile, ThemeTokens, builtin_tokens, copy, read_theme_file, slug};

use super::document::subject::subject_for;
use super::document::{Document, Subject};
use super::draft::draft_for;
use super::names::{today, untitled_name};
use super::themes::{Loaded, Themes};

/// A trusted project's theme file, read.
pub(super) struct ProjectTheme {
    pub(super) root: PathBuf,
    pub(super) text: String,
}

/// What the Studio opens from.
pub(super) struct Inputs {
    /// The pick the window shows now.
    pub(super) shown: String,
    /// The project's theme, when one is in effect and may be edited.
    pub(super) project: Option<ProjectTheme>,
    /// The theme in use, for what cannot be edited as itself.
    pub(super) in_use: ThemeTokens,
}

/// What the theme menu opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// A theme file of the themes folder.
    Own(PathBuf),
    /// A built-in, by name, as a copy to name.
    Builtin(String),
    /// A new theme: a copy of the theme in use.
    New,
    /// The project's theme.
    Project,
}

/// The document a menu choice opens; `None` when it can't be read.
pub(super) fn open_target(target: &Target, inputs: &Inputs, themes: &Themes) -> Option<Document> {
    match target {
        Target::Own(path) => {
            let Loaded { file, stamp } = (themes.read)(path).ok()?;
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
            Some(Document::new(named(file, stem), Subject::Own { path: path.clone() }, stamp))
        }
        Target::Builtin(name) => {
            let tokens = builtin_tokens().into_iter().find(|t| &t.name == name)?;
            Some(untitled(name, &tokens, themes))
        }
        Target::New => Some(untitled(&inputs.in_use.name, &inputs.in_use, themes)),
        Target::Project => {
            let project = inputs.project.as_ref()?;
            let file = read_theme_file(&project.text).ok()?;
            Some(Document::new(named(file, None), Subject::Project { root: project.root.clone() }, None))
        }
    }
}

pub(super) fn open_document(inputs: &Inputs, themes: &Themes) -> Document {
    let project = inputs.project.as_ref();
    let subject = subject_for(
        project.map(|p| p.root.clone()),
        &inputs.shown,
        &|setting| (themes.locate)(setting),
        &inputs.in_use.name,
    );
    let opened = match &subject {
        Subject::Own { path } => (themes.read)(path).ok().map(|Loaded { file, stamp }| {
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
            (named(file, stem), stamp)
        }),
        Subject::Project { .. } => {
            project.and_then(|p| read_theme_file(&p.text).ok()).map(|file| (named(file, None), None))
        }
        Subject::Untitled { .. } => None,
    };
    match opened {
        Some((file, stamp)) => Document::new(file, subject, stamp),
        None => {
            // A pick that is not a file we can edit: a copy of the theme in use.
            let from = match subject {
                Subject::Untitled { from } => from,
                _ => inputs.in_use.name.clone(),
            };
            untitled(&from, &inputs.in_use, themes)
        }
    }
}

/// A theme without a name of its own is called what its file is called.
fn named(mut file: ThemeFile, stem: Option<String>) -> ThemeFile {
    if file.name.as_deref().is_none_or(|n| n.trim().is_empty()) {
        file.name = stem.or_else(|| Some("Theme".to_string()));
    }
    file
}

/// "Untitled (from X)": a built-in as a copy (its colours in a `base`, nothing of its own on top),
/// or any other theme in use as a palette. Not dirty until the first edit.
fn untitled(from: &str, in_use: &ThemeTokens, themes: &Themes) -> Document {
    let subject = Subject::Untitled { from: from.to_string() };
    let base = slug::builtin_slug(from).and_then(|s| copy::copy_builtin_base(s, &|_| None, &today()).ok());
    let file = match base {
        Some(base) => ThemeFile {
            version: Some(wormhole_core::theme::tokens::CURRENT_VERSION),
            name: Some(untitled_name(from)),
            base: Some(Box::new(base)),
            ..ThemeFile::default()
        },
        None => ThemeFile { name: Some(untitled_name(from)), ..draft_for(in_use, themes.resolve) },
    };
    Document::new(file, subject, None)
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::tokens::{ThemeTokens, builtin_tokens, resolve};

    use super::*;
    use crate::studio::themes::{Found, Loaded};

    fn nord() -> ThemeTokens {
        builtin_tokens().into_iter().find(|t| t.name == "Nord").expect("Nord is built in")
    }

    fn themes() -> Themes {
        Themes {
            resolve: |f| resolve(f, &|_| None),
            saved_path: |_| None,
            version_named: |_| false,
            catalog: Vec::new,
            locate: |setting| match setting {
                "mine" => Found::File("/themes/mine.json".into()),
                "Nord" => Found::Builtin("Nord".into()),
                _ => Found::Missing,
            },
            read: |_| {
                let file = read_theme_file(
                    r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#112233"}}"##,
                )?;
                Ok(Loaded { file, stamp: None })
            },
            save_new: |_| Err("stub".into()),
            overwrite: |_, _, _| Err(crate::studio::themes::SaveError::Other("stub".into())),
            new_id: |name, _| slug::derive_id(name),
            convert: |file, id| {
                copy::convert_to_v1(file, &|_| None, &|n| slug::derive_id(n), id, "2026-10-29")
            },
            selects: |_, _| false,
        }
    }

    fn inputs(shown: &str) -> Inputs {
        Inputs { shown: shown.into(), project: None, in_use: nord() }
    }

    #[test]
    fn a_built_in_opens_as_an_untitled_copy_that_is_not_dirty() {
        let doc = open_document(&inputs("Nord"), &themes());
        assert_eq!(doc.subject, Subject::Untitled { from: "Nord".into() });
        assert_eq!(doc.name(), "Untitled (from Nord)");
        assert!(!doc.dirty());
        let base = doc.file.base.as_ref().expect("the copy step ran");
        assert_eq!(base.from, "nord");
        assert!(doc.file.ui.is_empty() && doc.file.id.is_none(), "nothing of its own yet");
        // It looks exactly like the theme it was copied from.
        let resolved = (themes().resolve)(&doc.file).expect("resolves");
        assert_eq!(resolved.terminal, nord().terminal);
    }

    #[test]
    fn the_persons_own_theme_is_edited_where_it_is() {
        let doc = open_document(&inputs("mine"), &themes());
        assert_eq!(doc.subject, Subject::Own { path: "/themes/mine.json".into() });
        assert_eq!(doc.name(), "Mine");
        assert_eq!(doc.file.ui.get("primary").map(String::as_str), Some("#112233"));
        assert!(!doc.dirty());
    }

    #[test]
    fn a_pick_that_cannot_be_edited_as_itself_is_a_copy_of_the_theme_in_use() {
        let doc = open_document(&inputs("gone"), &themes());
        assert_eq!(doc.subject, Subject::Untitled { from: "Nord".into() });
        assert_eq!(doc.name(), "Untitled (from Nord)");
        assert!(!doc.dirty());
    }

    #[test]
    fn a_project_theme_in_effect_is_edited_as_the_project_has_it() {
        let project =
            ProjectTheme { root: "/work/app".into(), text: r##"{"name": "App", "extends": "Nord"}"##.into() };
        let doc = open_document(&Inputs { project: Some(project), ..inputs("Nord") }, &themes());
        assert_eq!(doc.subject, Subject::Project { root: "/work/app".into() });
        assert_eq!(doc.name(), "App");
    }
}
