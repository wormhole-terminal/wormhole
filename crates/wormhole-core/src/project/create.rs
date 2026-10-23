//! Making a `.wormhole/` project folder from the Project tab: the one place the app creates a
//! folder inside someone's project rather than reading what is there.
//!
//! It starts from nothing and only from nothing. A folder that is already there (a link, a file
//! named `.wormhole`, a project) is the person's or the repository's, and is left exactly as it is.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    CONFIG_FILE, DIR_NAME, LinkPolicy, NOTES_FILE, NotesDoc, ProjectError, render_notes_md, write_atomic,
};

/// What a new `config.json` says: every key it can have, commented out, so nothing is set until
/// the person turns one on. `config.json` accepts comments, so it still loads as an empty file.
const CONFIG_TEMPLATE: &str = r##"// Settings for this project. Every key is optional: uncomment the ones you want.
// Commit this folder to share it; each person decides in Wormhole which parts to use.
{
  // The colour of this project's band in the terminal list and of its title row's tint: one of
  // red, orange, yellow, green, teal, blue, purple or pink (drawn from the theme), or a hex colour.
  // Wormhole's colour menus write this key.
  // "project_color": "teal",

  // Set to true to keep those menus from changing the colour: only editing this file does.
  // "project_color_locked": true,

  // Any setting that changes how terminals look here (fonts, the sidebar's rows, the diff …).
  // "font_size": 15,

  // Added to the environment of new terminals here. Wormhole asks before it takes effect.
  // "env": { "RUST_LOG": "debug" },

  // The program new terminals here start. Wormhole asks before it takes effect.
  // "shell": "/bin/zsh",

  // What a new worktree's terminal runs first, once (Shell → New Terminal in New Worktree…).
  // The worktree's own ports are in $WORMHOLE_PORT and $WORMHOLE_PORTS. Wormhole asks before it
  // takes effect.
  // "worktree": { "setup": "npm install" }
}
"##;

/// Make `folder/.wormhole/` with a `config.json` that sets nothing yet and an empty `notes.md`,
/// and return the new project folder. `folder` is the directory the project is for (a repository's
/// root, else a terminal's folder); it must exist.
///
/// The `notes.md` is there so the terminals in the new project sync their notes at once: Wormhole
/// syncs where the file is (or where the person switched Notes on, which makes it), and a project
/// folder without one would show Notes on while nothing synced. It holds the canonical empty
/// document, which is what the first sync would leave it as.
///
/// Refuses, and changes nothing, when `folder/.wormhole` is there already in any form: a project
/// is never replaced, and a link is never written through (`create_dir` fails on a link whatever
/// it points at, and the files go through `write_atomic` confined to `folder`). A failure after
/// the folder was made removes what was made, so no half-made `.wormhole/` is left to be taken for
/// a project.
pub fn init(folder: &Path) -> Result<PathBuf, ProjectError> {
    let notes = render_notes_md(&NotesDoc::default());
    make(folder, &[(CONFIG_FILE, CONFIG_TEMPLATE.as_bytes()), (NOTES_FILE, notes.as_bytes())])
}

/// [`init`] with the files to put in the new folder, in order.
fn make(folder: &Path, files: &[(&str, &[u8])]) -> Result<PathBuf, ProjectError> {
    let dir = folder.join(DIR_NAME);
    if !folder.is_dir() {
        return Err(ProjectError::invalid(folder, "is not a folder"));
    }
    match fs::create_dir(&dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            return Err(ProjectError::invalid(&dir, "already exists"));
        }
        Err(err) => return Err(ProjectError::io(&dir, err)),
    }
    let mut written: Vec<PathBuf> = Vec::new();
    for (name, bytes) in files {
        let path = dir.join(name);
        if let Err(err) = write_atomic(&path, bytes, LinkPolicy::Within(folder)) {
            // Only what we made a moment ago is in it.
            for done in written.iter().rev() {
                if let Err(cleanup) = fs::remove_file(done) {
                    tracing::warn!("removing the unfinished {}: {cleanup}", done.display());
                }
            }
            if let Err(cleanup) = fs::remove_dir(&dir) {
                tracing::warn!("removing the unfinished {}: {cleanup}", dir.display());
            }
            return Err(err);
        }
        written.push(path);
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{CONFIG_TEMPLATE, init, make};
    use crate::project::{
        CONFIG_FILE, DIR_NAME, Feature, NOTES_FILE, NotesDoc, ProjectFolder, find_project, parse_notes_md,
        plan_sync, render_notes_md,
    };

    fn scratch() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical");
        (tmp, root)
    }

    /// The names inside `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(dir)
            .expect("read dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }

    #[test]
    fn it_makes_the_folder_and_a_config_that_loads_and_sets_nothing() {
        let (_tmp, root) = scratch();
        let made = init(&root).expect("created");
        assert_eq!(made, root.join(DIR_NAME));
        assert_eq!(names(&made), [CONFIG_FILE, NOTES_FILE], "no temporary file is left behind");
        assert_eq!(fs::read_to_string(made.join(CONFIG_FILE)).expect("config"), CONFIG_TEMPLATE);

        // It is a project now: found from inside the folder, and everything in it loads.
        let sub = root.join("src");
        fs::create_dir(&sub).expect("mkdir");
        assert_eq!(find_project(&sub).map(|project| project.root), Some(root.clone()));
        let project = ProjectFolder::open(&root).expect("a project");
        let config = project.config().expect("loads").expect("present");
        assert!(config.settings.is_empty() && config.env.is_empty());
        assert!(config.shell.is_none() && config.color.is_none() && config.color_error.is_none());
        let snapshot = project.snapshot();
        assert_eq!(snapshot.errors().count(), 0);
        assert!(snapshot.warnings().is_empty());
        let wants = snapshot.trust_request().map(|request| request.commands.len());
        assert_eq!(wants, Some(0), "nothing in a new project can run, so there is nothing to trust");
    }

    /// A new project syncs notes at once: the file is there, it reads as no notes and no tasks, and
    /// the first sync of a terminal with none of its own has nothing to write.
    #[test]
    fn it_makes_an_empty_notes_file_so_notes_sync_right_away() {
        let (_tmp, root) = scratch();
        init(&root).expect("created");
        let project = ProjectFolder::open(&root).expect("a project");
        assert!(project.has(Feature::Notes));
        let on_disk = fs::read_to_string(project.notes_path()).expect("notes.md");
        assert_eq!(on_disk, render_notes_md(&NotesDoc::default()), "the canonical empty document");
        let doc = project.notes().expect("loads").expect("present");
        assert_eq!(doc, NotesDoc::default());
        assert_eq!(parse_notes_md(&on_disk), doc);
        let nothing = NotesDoc::default();
        assert!(plan_sync(None, &root, &nothing, &doc, false).is_none(), "nothing to write for nothing");
        // And a terminal with a note and a task writes them into it.
        let mut local = NotesDoc::default();
        local.notes.push("first".to_string());
        let plan = plan_sync(None, &root, &local, &doc, false).expect("a plan");
        assert_eq!(plan.write.as_ref(), Some(&local));
        assert!(project.write_notes(&local).expect("written"));
        assert_eq!(project.notes().expect("loads"), Some(local));
    }

    /// Uncommenting every key of the template, as the comments invite, is a file that loads.
    #[test]
    fn every_key_the_template_names_works_when_uncommented() {
        let (_tmp, root) = scratch();
        let made = init(&root).expect("created");
        let on: Vec<String> = CONFIG_TEMPLATE
            .lines()
            .map(|line| {
                line.strip_prefix("  // \"").map_or_else(|| line.to_string(), |key| format!("  \"{key}"))
            })
            .collect();
        fs::write(made.join(CONFIG_FILE), on.join("\n")).expect("write");
        let project = ProjectFolder::open(&root).expect("a project");
        let config = project.config().expect("loads").expect("present");
        assert!(config.color.is_some() && config.color_locked && config.lock_error.is_none());
        assert!(config.shell.is_some() && config.env.len() == 1);
        assert_eq!(config.settings.len(), 1, "font_size is the one setting: {:?}", config.settings);
    }

    #[test]
    fn an_existing_project_is_never_touched() {
        let (_tmp, root) = scratch();
        fs::create_dir_all(root.join(DIR_NAME)).expect("mkdir");
        fs::write(root.join(DIR_NAME).join(CONFIG_FILE), r#"{ "font_size": 20 }"#).expect("write");
        let err = init(&root).expect_err("refused");
        assert!(err.to_string().contains("already exists"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join(DIR_NAME).join(CONFIG_FILE)).expect("config"),
            r#"{ "font_size": 20 }"#
        );
        // An empty one counts as there too.
        let (_other, empty) = scratch();
        fs::create_dir(empty.join(DIR_NAME)).expect("mkdir");
        assert!(init(&empty).is_err());
        assert!(names(&empty.join(DIR_NAME)).is_empty(), "left as it was");
    }

    #[test]
    fn a_file_or_a_link_in_its_place_is_left_alone() {
        let (_tmp, root) = scratch();
        fs::write(root.join(DIR_NAME), "a file").expect("write");
        assert!(init(&root).is_err());
        assert_eq!(fs::read_to_string(root.join(DIR_NAME)).expect("still there"), "a file");
        {
            use crate::platform::fs::symlink;
            // A link to a folder somewhere else: nothing is written through it.
            let (_elsewhere_tmp, elsewhere) = scratch();
            let (_repo_tmp, repo) = scratch();
            symlink(&elsewhere, repo.join(DIR_NAME)).expect("symlink");
            assert!(init(&repo).is_err());
            assert!(names(&elsewhere).is_empty(), "the link's target got nothing");
            assert!(fs::symlink_metadata(repo.join(DIR_NAME)).expect("link").file_type().is_symlink());

            // A dangling one too: create_dir would otherwise make its target.
            let (_dangling_tmp, dangling) = scratch();
            let missing = elsewhere.join("missing");
            symlink(&missing, dangling.join(DIR_NAME)).expect("symlink");
            assert!(init(&dangling).is_err());
            assert!(!missing.exists(), "the link's target was not made");
        }
    }

    /// A write that fails after the first file was made takes the first file back too: an
    /// `.wormhole/` with a `config.json` and nothing else would be a project nobody asked for.
    #[test]
    fn a_failure_part_way_removes_everything_that_was_made() {
        let (_tmp, root) = scratch();
        // A name that can't be a file's, so the second write fails after the first went through.
        let err = make(&root, &[(CONFIG_FILE, b"{}"), ("bad\0name", b"x")]).expect_err("fails");
        drop(err);
        assert!(names(&root).is_empty(), "nothing left: {:?}", names(&root));
        // Nothing is in the way of trying again.
        assert!(init(&root).is_ok());
    }

    #[test]
    fn a_folder_that_is_not_there_gets_nothing() {
        let (_tmp, root) = scratch();
        let err = init(&root.join("gone")).expect_err("refused");
        assert!(err.to_string().contains("not a folder"), "{err}");
        assert!(names(&root).is_empty());
        fs::write(root.join("file"), "x").expect("write");
        assert!(init(&root.join("file")).is_err(), "a file is not a folder either");
    }

    #[test]
    fn a_folder_that_cannot_be_written_to_fails_without_leaving_a_half_made_project() {
        let (_tmp, root) = scratch();
        let locked = root.join("locked");
        fs::create_dir(&locked).expect("mkdir");
        fs::set_permissions(&locked, crate::platform::fs::permissions(0o555)).expect("chmod");
        // (Root can write anywhere; the check is for a person's own account.)
        let result = init(&locked);
        fs::set_permissions(&locked, crate::platform::fs::permissions(0o755)).expect("chmod back");
        if result.is_ok() {
            return;
        }
        assert!(names(&locked).is_empty());
    }
}
