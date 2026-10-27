//! The skill's format, and its install in a temp dir standing in for the home directory.

use std::path::Path;

use super::{
    Done, FILES, MARKER, NAME, State, TARGETS, covered_by, file, install, state, states, target, uninstall,
};
use crate::agents::AgentId;

fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn claude() -> &'static super::Target {
    target(AgentId::Claude).expect("Claude Code loads skills")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// The frontmatter of `SKILL.md`: the `key: value` lines between the two `---`, which must be the
/// file's first lines (the Agent Skills format, and what each agent's loader reads).
fn frontmatter(text: &str) -> Vec<(String, String)> {
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "the frontmatter starts the file");
    let mut fields = Vec::new();
    for line in lines {
        if line == "---" {
            return fields;
        }
        let (key, value) = line.split_once(": ").expect("a `key: value` line");
        fields.push((key.to_string(), value.to_string()));
    }
    panic!("the frontmatter never ends");
}

#[test]
fn the_frontmatter_is_valid_for_the_format() {
    let fields = frontmatter(file("SKILL.md").expect("SKILL.md").text);
    let get = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    let name = get("name").expect("a name");
    // agentskills.io: 1 to 64 characters, lowercase letters, digits and single hyphens, not at the ends,
    // and the folder's name.
    assert_eq!(name, NAME);
    assert!((1..=64).contains(&name.len()));
    assert!(name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'));
    assert!(!name.starts_with('-') && !name.ends_with('-') && !name.contains("--"));
    let description = get("description").expect("a description");
    assert!((1..=1024).contains(&description.chars().count()), "{}", description.chars().count());
    // A plain YAML scalar: nothing that would turn it into a mapping or a comment.
    assert!(!description.contains(": ") && !description.contains(" #"), "{description}");
    assert!(!description.starts_with(['"', '\'', '`', '[', '{', '>', '|', '&', '*', '!', '%', '@']));
    // What the description must say, for an agent to pick the skill at the right time.
    for word in ["Wormhole", "wh", "TUI", "dev server", "test", "agent"] {
        assert!(description.contains(word), "the description names {word}");
    }
    let keys: Vec<_> = fields.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["name", "description"], "only what every agent reads");
}

#[test]
fn the_files_are_short_marked_and_linked_to_each_other() {
    let skill = file("SKILL.md").expect("SKILL.md").text;
    assert!(skill.lines().count() < 150, "SKILL.md stays short: details are in the references");
    assert!(skill.contains(MARKER), "SKILL.md carries the marker");
    for other in FILES.iter().filter(|f| f.path != "SKILL.md") {
        assert!(skill.contains(&format!("({})", other.path)), "SKILL.md links {}", other.path);
        assert!(!other.text.contains(MARKER) || other.path == "SKILL.md");
        assert!(other.text.lines().count() < 500);
    }
    // One level deep, relative: no absolute paths, no links out.
    assert!(FILES.iter().all(|f| !f.text.contains("](/")));
}

#[test]
fn install_writes_the_files_and_is_idempotent() {
    let home = home();
    assert_eq!(state(claude(), home.path()), Ok(State::NotInstalled));
    assert_eq!(install(claude(), home.path()), Ok(Done::Installed));
    assert_eq!(state(claude(), home.path()), Ok(State::Current));
    for file in FILES {
        let written = std::fs::read_to_string(claude().path(home.path()).join(file.path)).expect("written");
        assert_eq!(written, file.text);
    }
    assert_eq!(install(claude(), home.path()), Ok(Done::Unchanged));
    // Nothing else of the home was made.
    let top: Vec<_> =
        std::fs::read_dir(home.path()).expect("home").flatten().map(|e| e.file_name()).collect();
    assert_eq!(top, [std::ffi::OsString::from(".claude")]);
}

#[test]
fn an_older_copy_of_ours_is_updated_and_a_stale_one_is_told() {
    let home = home();
    install(claude(), home.path()).expect("installs");
    let skill_md = claude().path(home.path()).join("SKILL.md");
    let old = std::fs::read_to_string(&skill_md)
        .expect("read")
        .replace("Driving Wormhole", "Driving Wormhole (old)");
    std::fs::write(&skill_md, old).expect("age it");
    assert_eq!(state(claude(), home.path()), Ok(State::Outdated));
    assert_eq!(install(claude(), home.path()), Ok(Done::Updated));
    assert_eq!(state(claude(), home.path()), Ok(State::Current));
    // A missing reference counts as out of date too.
    std::fs::remove_file(claude().path(home.path()).join("references/errors.md")).expect("remove");
    assert_eq!(state(claude(), home.path()), Ok(State::Outdated));
}

#[test]
fn another_skill_of_the_same_name_is_never_touched() {
    let home = home();
    let skill_md = claude().path(home.path()).join("SKILL.md");
    write(&skill_md, "---\nname: wormhole\ndescription: mine\n---\nmy own notes\n");
    assert_eq!(state(claude(), home.path()), Ok(State::Foreign));
    let err = install(claude(), home.path()).expect_err("refused");
    assert!(err.starts_with("Another skill is at "), "{err}");
    assert!(uninstall(claude(), home.path()).is_err());
    assert_eq!(
        std::fs::read_to_string(&skill_md).expect("read"),
        "---\nname: wormhole\ndescription: mine\n---\nmy own notes\n"
    );
    // A folder with files but no SKILL.md is somebody's too; an empty one is nobody's.
    let other = tempfile::tempdir().expect("tempdir");
    write(&claude().path(other.path()).join("notes.txt"), "x");
    assert_eq!(state(claude(), other.path()), Ok(State::Foreign));
    let empty = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(claude().path(empty.path())).expect("mkdir");
    assert_eq!(state(claude(), empty.path()), Ok(State::NotInstalled));
    assert_eq!(install(claude(), empty.path()), Ok(Done::Installed));
}

#[test]
fn a_link_is_foreign_and_a_link_inside_cannot_send_a_write_out() {
    let home = home();
    let outside = tempfile::tempdir().expect("tempdir");
    let dir = claude().path(home.path());
    std::fs::create_dir_all(dir.parent().expect("parent")).expect("mkdir");
    crate::platform::fs::symlink(outside.path(), &dir).expect("link the whole folder");
    assert_eq!(state(claude(), home.path()), Ok(State::Foreign));
    assert!(install(claude(), home.path()).is_err());
    assert_eq!(std::fs::read_dir(outside.path()).expect("outside").count(), 0, "nothing written through it");

    // Ours, but `references` is a link to somewhere else: the update must not write through it.
    let home = self::home();
    install(claude(), home.path()).expect("installs");
    let dir = claude().path(home.path());
    std::fs::remove_dir_all(dir.join("references")).expect("remove");
    crate::platform::fs::symlink(outside.path(), dir.join("references")).expect("link references");
    let _ = install(claude(), home.path());
    assert_eq!(std::fs::read_dir(outside.path()).expect("outside").count(), 0, "nothing written through it");
}

#[test]
fn uninstall_removes_only_what_we_wrote() {
    let home = home();
    assert_eq!(uninstall(claude(), home.path()), Ok(false));
    install(claude(), home.path()).expect("installs");
    let dir = claude().path(home.path());
    write(&dir.join("my-notes.md"), "keep");
    assert_eq!(uninstall(claude(), home.path()), Ok(true));
    assert!(!dir.join("SKILL.md").exists() && !dir.join("references").exists());
    assert_eq!(std::fs::read_to_string(dir.join("my-notes.md")).expect("kept"), "keep");
    // With nothing of the person's in it, the folder goes too, and the skills folder above stays.
    let clean = self::home();
    install(claude(), clean.path()).expect("installs");
    assert_eq!(uninstall(claude(), clean.path()), Ok(true));
    assert!(!claude().path(clean.path()).exists());
    assert!(clean.path().join(".claude/skills").is_dir());
}

#[test]
fn every_target_has_its_own_folder_and_its_agent_is_named_by_one_word() {
    let mut words: Vec<_> = TARGETS.iter().map(|t| t.word()).collect();
    assert!(words.iter().all(|w| !w.is_empty()));
    words.sort();
    words.dedup();
    assert_eq!(words.len(), TARGETS.len());
    let mut dirs: Vec<_> = TARGETS.iter().map(|t| t.dir).collect();
    dirs.sort_unstable();
    dirs.dedup();
    assert_eq!(
        dirs.len(),
        TARGETS.len(),
        "two agents would share one folder, and one Uninstall would take both"
    );
    for target in TARGETS {
        assert!(target.dir.ends_with("skills") && !target.dir.starts_with('/'));
        assert!(target.source.starts_with("https://"));
    }
    assert_eq!(super::targets::by_word("OpenCode").map(|t| t.agent), Some(AgentId::OpenCode));
    // Everything installs into a scratch home and reads back as current.
    let home = home();
    for target in TARGETS {
        install(target, home.path()).expect("installs");
    }
    assert!(states(home.path()).iter().all(|(_, state)| *state == Ok(State::Current)));
}

#[test]
fn an_agent_that_reads_a_folder_with_our_current_copy_is_covered() {
    let home = home();
    let of = |agent| target(agent).expect("loads skills");
    assert_eq!(covered_by(of(AgentId::Gemini), home.path()), None);
    install(of(AgentId::Codex), home.path()).expect("installs in the shared folder");
    for agent in [AgentId::Gemini, AgentId::Copilot, AgentId::OpenCode, AgentId::Qwen, AgentId::Droid] {
        assert_eq!(covered_by(of(agent), home.path()).as_deref(), Some("~/.agents/skills"), "{agent:?}");
    }
    // Claude and Codex read only their own.
    assert_eq!(covered_by(of(AgentId::Claude), home.path()), None);
    assert_eq!(covered_by(of(AgentId::Codex), home.path()), None);
    // OpenCode also reads Claude's folder.
    let other = self::home();
    install(of(AgentId::Claude), other.path()).expect("installs");
    assert_eq!(covered_by(of(AgentId::OpenCode), other.path()).as_deref(), Some("~/.claude/skills"));
    assert_eq!(covered_by(of(AgentId::Gemini), other.path()), None, "Gemini does not read Claude's");
    // An out-of-date copy is not the current skill: it does not cover.
    let old = claude().path(other.path()).join("SKILL.md");
    let text = std::fs::read_to_string(&old).expect("read").replace("Driving Wormhole", "Driving");
    std::fs::write(&old, text).expect("age it");
    assert_eq!(covered_by(of(AgentId::OpenCode), other.path()), None);
}
