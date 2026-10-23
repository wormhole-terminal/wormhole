//! Changing a project's colour from the app: the one edit Wormhole makes to a project's own
//! `config.json` (the colour menus of a folder's band and the Project tab).
//!
//! The file is a repository's and was written by hand, so the edit is a splice into its text
//! (`jsonc::set_member`), never a rewrite: comments, other keys and layout stay exactly as they
//! were. It goes through the same capped, root-bound read as loading and the same atomic,
//! root-bound write as every other file Wormhole puts in a project, and it never touches a file it
//! could not read in full.

use serde_json::{Map, Value};

use super::color::{KEY, LOCK_KEY};
use super::{
    CONFIG_FILE, LinkPolicy, MAX_JSON_BYTES, ProjectColorChoice, ProjectError, ProjectFolder, write_atomic,
};
use crate::storage::jsonc;
use crate::theme::folder_color::FolderColor;

impl ProjectFolder {
    /// Write `"project_color": "<name>"` into `config.json`, or (`None`) take the key out, and
    /// leave everything else in the file as it is. Returns whether the file changed: choosing what
    /// is already there writes nothing, so git and the file watchers see no edit.
    ///
    /// Refuses, and leaves the file alone, when it isn't there, can't be read (a link leaving the
    /// project, too large), isn't a JSON object that reads in full, or when the project has locked
    /// its colour (`"project_color_locked": true`): the lock is read from the file as it is now,
    /// so a menu that was opened before the lock was added can't get past it.
    pub fn set_project_color(&self, color: Option<FolderColor>) -> Result<bool, ProjectError> {
        self.set_project_color_choice(color.map(ProjectColorChoice::Named))
    }

    /// [`Self::set_project_color`] for a name or a hex colour the person picked.
    pub fn set_project_color_choice(&self, color: Option<ProjectColorChoice>) -> Result<bool, ProjectError> {
        let path = self.path(CONFIG_FILE);
        let text = self
            .read_text(&path, MAX_JSON_BYTES)?
            .ok_or_else(|| ProjectError::invalid(&path, "does not exist"))?;
        let cannot = |why: String| ProjectError::invalid(&path, format!("can't change the colour: {why}"));
        let current: Map<String, Value> = jsonc::parse(&text).map_err(cannot)?;
        if current.get(LOCK_KEY) == Some(&Value::Bool(true)) {
            return Err(ProjectError::invalid(
                &path,
                format!("the project locks its colour ({LOCK_KEY}); edit the file to change it"),
            ));
        }
        let value = color.map(|color| Value::String(color.to_value()));
        let edited = jsonc::set_member(&text, KEY, value.as_ref()).map_err(cannot)?;
        if edited == text {
            return Ok(false);
        }
        write_atomic(&path, edited.as_bytes(), LinkPolicy::Within(&self.root))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::tests::project;
    use super::super::{CONFIG_FILE, ProjectColorChoice, ProjectFolder, init};
    use crate::storage::fsutil::FileStamp;
    use crate::theme::folder_color::FolderColor;

    fn config_of(project: &ProjectFolder) -> String {
        fs::read_to_string(project.path(CONFIG_FILE)).expect("config.json")
    }

    fn color_of(project: &ProjectFolder) -> Option<ProjectColorChoice> {
        project.config().expect("loads").expect("present").color
    }

    #[test]
    fn it_writes_a_picked_hex_colour_that_reads_back() {
        let (_tmp, p) = project(&[(CONFIG_FILE, "{}\n")]);
        let picked = ProjectColorChoice::parse("#2bb3a3").expect("a colour");
        assert!(p.set_project_color_choice(Some(picked)).expect("written"));
        assert!(config_of(&p).contains("\"project_color\": \"#2bb3a3\""));
        assert_eq!(color_of(&p), Some(picked));
    }

    #[test]
    fn it_puts_a_name_into_a_file_and_leaves_the_rest_alone() {
        let text = "// Wormhole project\n{\n  \"font_size\": 15, // big\n  /* the shell */\n  \"shell\": \"/bin/zsh\"\n}\n";
        let (_tmp, p) = project(&[(CONFIG_FILE, text)]);
        assert!(p.set_project_color(Some(FolderColor::Teal)).expect("written"));
        assert_eq!(
            config_of(&p),
            "// Wormhole project\n{\n  \"font_size\": 15, // big\n  /* the shell */\n  \"shell\": \"/bin/zsh\",\n  \"project_color\": \"teal\"\n}\n"
        );
        assert_eq!(color_of(&p), Some(ProjectColorChoice::Named(FolderColor::Teal)));

        // Another colour replaces it where it stands; taking it away gives the file back.
        assert!(p.set_project_color(Some(FolderColor::Pink)).expect("written"));
        assert!(config_of(&p).contains("\"project_color\": \"pink\""));
        assert_eq!(color_of(&p), Some(ProjectColorChoice::Named(FolderColor::Pink)));
        assert!(p.set_project_color(None).expect("written"));
        assert_eq!(config_of(&p), text);
        assert_eq!(color_of(&p), None);
    }

    /// A hand-written hex colour is replaced by the name picked, and a name with a hex's trailing
    /// comment keeps the comment.
    #[test]
    fn it_replaces_a_hex_colour_and_keeps_its_comment() {
        let (_tmp, p) = project(&[(
            CONFIG_FILE,
            "{\n  \"project_color\": \"#2bb3a3\", // our teal\n  \"font_size\": 15\n}\n",
        )]);
        p.set_project_color(Some(FolderColor::Blue)).expect("written");
        assert_eq!(config_of(&p), "{\n  \"project_color\": \"blue\", // our teal\n  \"font_size\": 15\n}\n");
    }

    /// What `project::init` writes: the commented-out sample is a comment, so the real key is
    /// added beside the other comments, all of which stay.
    #[test]
    fn it_adds_the_key_to_the_template_init_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical");
        init(&root).expect("created");
        let p = ProjectFolder::open(&root).expect("a project");
        let template = config_of(&p);
        assert!(template.contains("// \"project_color\":"), "the sample is commented out");
        p.set_project_color(Some(FolderColor::Green)).expect("written");
        let edited = config_of(&p);
        for line in template.lines() {
            assert!(edited.lines().any(|l| l == line), "{line:?} is gone from {edited}");
        }
        assert_eq!(
            edited.lines().count(),
            template.lines().count() + 2,
            "the key and the blank line above it"
        );
        assert!(edited.contains("\n  \"project_color\": \"green\"\n}"), "{edited}");
        assert_eq!(color_of(&p), Some(ProjectColorChoice::Named(FolderColor::Green)));
        p.set_project_color(None).expect("written");
        assert_eq!(color_of(&p), None);
        assert!(!config_of(&p).contains("\"project_color\": \"green\""));
    }

    #[test]
    fn choosing_what_is_already_there_writes_nothing() {
        let (_tmp, p) = project(&[(CONFIG_FILE, "{ \"project_color\": \"red\" }")]);
        let stamp = || FileStamp::of(&p.path(CONFIG_FILE));
        let before = stamp();
        assert!(!p.set_project_color(Some(FolderColor::Red)).expect("fine"));
        assert_eq!(stamp(), before, "the file was not replaced");
        // Nor does taking away a colour that isn't there.
        let (_tmp, bare) = project(&[(CONFIG_FILE, "{ \"font_size\": 15 }")]);
        let before = FileStamp::of(&bare.path(CONFIG_FILE));
        assert!(!bare.set_project_color(None).expect("fine"));
        assert_eq!(FileStamp::of(&bare.path(CONFIG_FILE)), before);
    }

    /// The lock is read from the file as it is when the edit is made.
    #[test]
    fn a_locked_colour_is_not_changed() {
        let locked = "{\n  \"project_color\": \"red\",\n  \"project_color_locked\": true\n}\n";
        let (_tmp, p) = project(&[(CONFIG_FILE, locked)]);
        for color in [Some(FolderColor::Teal), None] {
            let err = p.set_project_color(color).expect_err("locked");
            assert!(err.to_string().contains("locks its colour"), "{err}");
            assert_eq!(config_of(&p), locked, "the file is as it was");
        }
        // Only `true` locks; the file loads the same way.
        for (text, locks) in [
            ("{ \"project_color_locked\": false }", false),
            ("{ \"project_color_locked\": null }", false),
            ("{ \"project_color_locked\": \"true\" }", false),
            ("{ \"project_color_locked\": 1 }", false),
            ("{ \"project_color_locked\": true }", true),
        ] {
            let (_tmp, p) = project(&[(CONFIG_FILE, text)]);
            assert_eq!(p.config().expect("loads").expect("present").color_locked, locks, "{text}");
            assert_eq!(p.set_project_color(Some(FolderColor::Teal)).is_err(), locks, "{text}");
        }
    }

    #[test]
    fn a_file_that_does_not_read_in_full_is_never_rewritten() {
        for text in [
            "",
            "[]",
            "// only a comment\n",
            "{",
            "{\"font_size\": }",
            "{ \"a\": 1 } trailing",
            "{ \"a\": \"unterminated }",
            "{ /* never closed }",
        ] {
            let (_tmp, p) = project(&[(CONFIG_FILE, text)]);
            let err = p.set_project_color(Some(FolderColor::Teal)).expect_err("refused");
            assert!(err.to_string().contains("can't change the colour"), "{text:?}: {err}");
            assert_eq!(config_of(&p), text, "{text:?} was rewritten");
        }
    }

    #[test]
    fn a_missing_file_is_not_made() {
        let (_tmp, p) = project(&[]);
        let err = p.set_project_color(Some(FolderColor::Teal)).expect_err("no config.json");
        assert!(err.to_string().contains("does not exist"), "{err}");
        assert!(!p.path(CONFIG_FILE).exists());
    }

    #[test]
    fn a_file_too_large_to_read_is_left_alone() {
        let big = format!("{{ \"a\": \"{}\" }}", "x".repeat(1024 * 1024));
        let (_tmp, p) = project(&[(CONFIG_FILE, &big)]);
        assert!(p.set_project_color(Some(FolderColor::Teal)).is_err());
        assert_eq!(config_of(&p).len(), big.len());
    }

    /// A repository can commit `config.json` as a link to a file elsewhere: neither read nor
    /// written through.
    #[test]
    fn a_link_out_of_the_project_is_not_written_through() {
        let (_tmp, p) = project(&[]);
        let elsewhere = TempDir::new().expect("tempdir");
        let target = elsewhere.path().join("victim.json");
        fs::write(&target, "{ \"keep\": true }").expect("write");
        crate::platform::fs::symlink(&target, p.path(CONFIG_FILE)).expect("symlink");
        assert!(p.set_project_color(Some(FolderColor::Teal)).is_err());
        assert_eq!(fs::read_to_string(&target).expect("read"), "{ \"keep\": true }");
        assert!(fs::symlink_metadata(p.path(CONFIG_FILE)).expect("still a link").file_type().is_symlink());
    }

    /// The write is atomic and leaves nothing beside the file.
    #[test]
    fn no_temporary_file_is_left_behind() {
        let (_tmp, p) = project(&[(CONFIG_FILE, "{}")]);
        p.set_project_color(Some(FolderColor::Teal)).expect("written");
        let names: Vec<String> = fs::read_dir(&p.dir)
            .expect("dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [CONFIG_FILE]);
    }
}
