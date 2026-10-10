//! Trust and the colour: choosing a colour never sends a trusted project back for approval, nothing
//! else in `config.json` is let through, and approvals given before the colour left the fingerprint
//! still hold.

use std::fs;

use tempfile::TempDir;

use super::super::tests::project;
use super::super::{
    ACTIONS_FILE, CONFIG_FILE, Feature, LAYOUT_FILE, ProjectSnapshot, describe_trust_request,
};
use super::{TrustStatus, TrustStore};

const ACTIONS: &str = r#"[{"id": "t", "title": "Test", "type": "run", "command": "cargo test"}]"#;

// ---- the colour is not part of what is approved ---------------------------------------------

const COLOUR_CONFIG: &str = "// Wormhole project\n{\n  \"font_size\": 13,\n  \"env\": { \"A\": \"1\" },\n  \"shell\": \"/bin/zsh\",\n  \"project_color\": \"#2bb3a3\"\n}\n";

/// A project with everything that can run, trusted: `config.json` is `config`.
fn trusted_with(config: &str) -> (TempDir, TrustStore, TempDir, crate::project::ProjectFolder) {
    let data = TempDir::new().expect("tempdir");
    let (tmp, p) = project(&[(CONFIG_FILE, config), (ACTIONS_FILE, ACTIONS)]);
    let mut store = TrustStore::load(data.path()).expect("load");
    store.trust(&p);
    assert_eq!(store.status(&p), TrustStatus::Trusted);
    (data, store, tmp, p)
}

/// Choosing, changing, removing and locking the colour, by the app or by hand, leaves a trusted
/// project trusted: it only paints.
#[test]
fn a_colour_change_keeps_a_trusted_project_trusted() {
    let (_data, store, _tmp, p) = trusted_with(COLOUR_CONFIG);
    let edits = [
        // The app's own edits.
        COLOUR_CONFIG.replace("\"#2bb3a3\"", "\"pink\""),
        COLOUR_CONFIG.replace("  \"project_color\": \"#2bb3a3\"\n", ""),
        COLOUR_CONFIG.replace(",\n  \"project_color\": \"#2bb3a3\"", ""),
        // By hand: a new value, a bad one, the lock, both.
        COLOUR_CONFIG.replace("#2bb3a3", "#000000"),
        COLOUR_CONFIG.replace("\"#2bb3a3\"", "\"not a colour\""),
        COLOUR_CONFIG.replace("\"#2bb3a3\"", "42"),
        COLOUR_CONFIG.replace("}\n", ",\n  \"project_color_locked\": true\n}\n"),
        COLOUR_CONFIG.replace("}\n", ",\n  \"project_color_locked\": \"yes\"\n}\n"),
    ];
    for edited in edits {
        fs::write(p.path(CONFIG_FILE), &edited).expect("write");
        assert_eq!(store.status(&p), TrustStatus::Trusted, "{edited}");
        assert_eq!(store.status_of(&ProjectSnapshot::load(&p)), TrustStatus::Trusted, "{edited}");
    }
    // The same through the app's own edit.
    fs::write(p.path(CONFIG_FILE), COLOUR_CONFIG).expect("write");
    for color in
        [Some(crate::folder_color::FolderColor::Red), Some(crate::folder_color::FolderColor::Blue), None]
    {
        p.set_project_color(color).expect("edited");
        assert_eq!(store.status(&p), TrustStatus::Trusted, "{color:?}");
    }
}

/// Anything else in `config.json` still needs approving: a setting, the environment, the shell,
/// a key a later version might read, a name that only looks like the colour's.
#[test]
fn any_other_change_to_config_json_needs_approving_again() {
    let (_data, store, _tmp, p) = trusted_with(COLOUR_CONFIG);
    let edits = [
        COLOUR_CONFIG.replace("13", "14"),
        COLOUR_CONFIG.replace("\"A\": \"1\"", "\"A\": \"2\""),
        COLOUR_CONFIG.replace("\"A\": \"1\"", "\"A\": \"1\", \"B\": \"2\""),
        COLOUR_CONFIG.replace("/bin/zsh", "/bin/bash"),
        COLOUR_CONFIG.replace("  \"shell\": \"/bin/zsh\",\n", ""),
        COLOUR_CONFIG.replace("}\n", ",\n  \"theme\": \"nord\"\n}\n"),
        COLOUR_CONFIG.replace("}\n", ",\n  \"project_colour\": \"teal\"\n}\n"),
        COLOUR_CONFIG.replace("}\n", ",\n  \"Project_Color\": \"teal\"\n}\n"),
        // The paint keys are only the top level's: the same names inside `env` are variables.
        COLOUR_CONFIG.replace("\"A\": \"1\"", "\"A\": \"1\", \"project_color\": \"x\""),
        COLOUR_CONFIG.replace("}\n", ",\n  \"settings\": { \"project_color\": \"x\" }\n}\n"),
        // The file goes away, or stops being JSON.
        String::new(),
        "{".to_string(),
        "[]".to_string(),
    ];
    for edited in edits {
        fs::write(p.path(CONFIG_FILE), &edited).expect("write");
        assert_eq!(store.status(&p), TrustStatus::Changed, "{edited}");
    }
    fs::remove_file(p.path(CONFIG_FILE)).expect("rm");
    assert_eq!(store.status(&p), TrustStatus::Changed);
}

/// What reaches the loader is what is approved: comments, white space and the order of keys
/// don't (they can't run anything), and the same text hashes the same wherever it is.
#[test]
fn config_json_is_approved_by_what_it_says_not_how_it_is_written() {
    let (_data, store, _tmp, p) = trusted_with(COLOUR_CONFIG);
    for same in [
        "{\"project_color\":\"teal\",\"shell\":\"/bin/zsh\",\"env\":{\"A\":\"1\"},\"font_size\":13}",
        "{ /* c */ \"font_size\": 13 ,\n \"shell\": \"/bin/zsh\", \"env\": {\"A\": \"1\",}, }",
        "{ \"env\": {\"A\": \"\\u0031\"}, \"font_size\": 13, \"shell\": \"\\/bin\\/zsh\" }",
    ] {
        fs::write(p.path(CONFIG_FILE), same).expect("write");
        let snapshot = ProjectSnapshot::load(&p);
        assert_eq!(store.status_of(&snapshot), TrustStatus::Trusted, "{same}");
        assert_eq!(snapshot.config().expect("loads").shell.as_deref(), Some("/bin/zsh"), "{same}");
    }
    // A number is compared as the loader reads it, and 13.0 is a float where 13 is an integer:
    // spelled the other way it counts as a change (the safe side of the same value).
    fs::write(p.path(CONFIG_FILE), COLOUR_CONFIG.replace("13", "13.0")).expect("write");
    assert_eq!(store.status(&p), TrustStatus::Changed);
    // A key twice reads as the last one, and that is what is compared.
    fs::write(
        p.path(CONFIG_FILE),
        COLOUR_CONFIG.replace("\"font_size\": 13,", "\"font_size\": 99, \"font_size\": 13,"),
    )
    .expect("write");
    assert_eq!(store.status(&p), TrustStatus::Trusted);
    // Array order and nesting are content.
    let (_data, store, _tmp, p) = trusted_with(r#"{ "tags": [1, 2], "o": { "a": { "b": 1 } } }"#);
    for (text, status) in [
        (r#"{ "o": { "a": { "b": 1 } }, "tags": [1, 2] }"#, TrustStatus::Trusted),
        (r#"{ "tags": [2, 1], "o": { "a": { "b": 1 } } }"#, TrustStatus::Changed),
        (r#"{ "tags": [1, 2], "o": { "a": { "b": 2 } } }"#, TrustStatus::Changed),
        (r#"{ "tags": [1, 2], "o": { "a": {}, "b": 1 } }"#, TrustStatus::Changed),
        (r#"{ "tags": [1, 2], "o": { "a": { "b": "1" } } }"#, TrustStatus::Changed),
        (r#"{ "tags": [1, 2], "o": { "a": { "b": 1 } }, "x": null }"#, TrustStatus::Changed),
    ] {
        fs::write(p.path(CONFIG_FILE), text).expect("write");
        assert_eq!(store.status(&p), status, "{text}");
    }
}

/// Neither the colour nor the lock approves anything, and a changed project stays changed.
#[test]
fn a_colour_change_approves_nothing() {
    let data = TempDir::new().expect("tempdir");
    let (_tmp, p) = project(&[(CONFIG_FILE, COLOUR_CONFIG), (ACTIONS_FILE, ACTIONS)]);
    let mut store = TrustStore::load(data.path()).expect("load");
    assert_eq!(store.status(&p), TrustStatus::Untrusted);
    p.set_project_color(Some(crate::folder_color::FolderColor::Teal)).expect("edited");
    p.set_project_color(None).expect("edited");
    assert_eq!(store.status(&p), TrustStatus::Untrusted, "never asked, still not trusted");
    assert!(!store.allows(&p, Feature::Actions) && !store.allows(&p, Feature::Settings));

    store.trust(&p);
    fs::write(p.path(CONFIG_FILE), COLOUR_CONFIG.replace("/bin/zsh", "/bin/evil")).expect("write");
    assert_eq!(store.status(&p), TrustStatus::Changed);
    p.set_project_color(Some(crate::folder_color::FolderColor::Pink)).expect("edited");
    assert_eq!(store.status(&p), TrustStatus::Changed, "a colour doesn't undo what changed");
    // Putting the approved content back (whatever the colour) is trusted again.
    fs::write(p.path(CONFIG_FILE), COLOUR_CONFIG.replace("#2bb3a3", "#ffffff")).expect("write");
    assert_eq!(store.status(&p), TrustStatus::Trusted);
    // And what the dialog was shown is what was approved, colour aside.
    let request = describe_trust_request(&p).expect("describe");
    fs::write(p.path(CONFIG_FILE), COLOUR_CONFIG.replace("#2bb3a3", "red")).expect("write");
    let mut fresh = TrustStore::load(TempDir::new().expect("tempdir").path()).expect("load");
    fresh.trust_request(&request);
    assert_eq!(fresh.status(&p), TrustStatus::Trusted);
}

// ---- approvals given before the colour left the fingerprint ---------------------------------

const GOLDEN_ACTIONS: &str = r#"[{"id": "go", "title": "Go", "type": "prompt", "file": "go.md", "submit": true}, {"id": "t", "title": "T", "type": "run", "command": "cargo test"}]"#;
const GOLDEN_LAYOUT: &str = r#"{"sessions": [{"name": "Claude", "run": "claude"}]}"#;

/// The fingerprints the previous build wrote (taken from it, not from this code): the old form
/// stays reproducible, which is what keeps every approval already given.
#[test]
fn the_old_fingerprint_is_still_computed_exactly_as_before() {
    let (_tmp, p) = project(&[
        (CONFIG_FILE, COLOUR_CONFIG),
        (ACTIONS_FILE, GOLDEN_ACTIONS),
        (LAYOUT_FILE, GOLDEN_LAYOUT),
        ("prompts/go.md", "Summarize the diff.\n"),
    ]);
    let snapshot = ProjectSnapshot::load(&p);
    assert_eq!(
        snapshot.legacy_fingerprint(),
        "1ed6f36638618d4af08699099bdddbb771016a60287b665153e94a29d0250d1e"
    );
    assert_ne!(snapshot.fingerprint(), snapshot.legacy_fingerprint());
    fs::remove_file(p.path(ACTIONS_FILE)).expect("rm");
    fs::remove_file(p.path(LAYOUT_FILE)).expect("rm");
    assert_eq!(
        ProjectSnapshot::load(&p).legacy_fingerprint(),
        "28e2e65fd85b6a2c9582f8adf71c9a9949536bf9e58316dee803340501eb665d"
    );
    // A file that doesn't load has no canonical form: both are its bytes.
    fs::write(p.path(CONFIG_FILE), "{\"font_size\": 13, \"oops\": ").expect("write");
    assert_eq!(
        ProjectSnapshot::load(&p).legacy_fingerprint(),
        "08a8dcaea3ec6e763d481eb8431a929e24eee5d6c07f433c94ff2ae094c57f92"
    );
}

/// An approval stored by the previous build is trusted for the bytes it covered, and only those;
/// rewriting it in the new form (saved) is what lets a colour change after it.
#[test]
fn an_approval_from_before_still_holds_and_is_rewritten() {
    let data = TempDir::new().expect("tempdir");
    let (_tmp, p) = project(&[(CONFIG_FILE, COLOUR_CONFIG), (ACTIONS_FILE, ACTIONS)]);
    let old = ProjectSnapshot::load(&p).legacy_fingerprint().to_string();
    // What the previous build left in trusted-projects.json.
    let key = super::super::features::root_key(&p.root);
    fs::write(
        data.path().join(TrustStore::FILE_NAME),
        format!(r#"{{ {key:?}: {{ "fingerprint": "{old}", "trusted_at": 1700000000 }} }}"#),
    )
    .expect("write");
    let mut store = TrustStore::load(data.path()).expect("load");
    assert_eq!(store.status(&p), TrustStatus::Trusted, "not asked again after the upgrade");
    assert_eq!(store.list()[0].fingerprint, old, "not rewritten until someone loads the project");

    let snapshot = ProjectSnapshot::load(&p);
    assert!(store.upgrade(&snapshot), "rewritten");
    assert_eq!(store.list()[0].fingerprint, snapshot.fingerprint());
    assert_eq!(store.list()[0].trusted_at, 1_700_000_000, "approved when it was approved");
    assert!(!store.upgrade(&snapshot), "once");
    store.save().expect("saved");

    // Now the colour can change, and the approval is still there after a restart.
    p.set_project_color(Some(crate::folder_color::FolderColor::Teal)).expect("edited");
    assert_eq!(TrustStore::load(data.path()).expect("load").status(&p), TrustStatus::Trusted);
    assert_eq!(store.status(&p), TrustStatus::Trusted);
}

/// The old form is accepted for the bytes it was given for, and for nothing else.
#[test]
fn an_old_approval_does_not_cover_what_changed_since() {
    let data = TempDir::new().expect("tempdir");
    let (_tmp, p) = project(&[(CONFIG_FILE, COLOUR_CONFIG), (ACTIONS_FILE, ACTIONS)]);
    let old = ProjectSnapshot::load(&p).legacy_fingerprint().to_string();
    let key = super::super::features::root_key(&p.root);
    fs::write(
        data.path().join(TrustStore::FILE_NAME),
        format!(r#"{{ {key:?}: {{ "fingerprint": "{old}", "trusted_at": 1 }} }}"#),
    )
    .expect("write");
    let mut store = TrustStore::load(data.path()).expect("load");
    for edited in [COLOUR_CONFIG.replace("/bin/zsh", "/bin/bash"), COLOUR_CONFIG.replace("#2bb3a3", "red")] {
        fs::write(p.path(CONFIG_FILE), &edited).expect("write");
        let snapshot = ProjectSnapshot::load(&p);
        assert_eq!(
            store.status_of(&snapshot),
            TrustStatus::Changed,
            "an old approval covers its own bytes: {edited}"
        );
        assert!(!store.upgrade(&snapshot), "{edited}");
        assert_eq!(store.list()[0].fingerprint, old, "left as it was: {edited}");
    }
    // Not approved at all: nothing to upgrade.
    let mut empty = TrustStore::load(TempDir::new().expect("tempdir").path()).expect("load");
    assert!(!empty.upgrade(&ProjectSnapshot::load(&p)));
    assert!(empty.list().is_empty());
    // Another project's approval is not this one's.
    let (_other_tmp, other) = project(&[(CONFIG_FILE, COLOUR_CONFIG), (ACTIONS_FILE, ACTIONS)]);
    assert_eq!(store.status(&other), TrustStatus::Untrusted);
}

/// The two forms can't be mistaken for each other: a new approval is not read as an old one,
/// whatever the content.
#[test]
fn the_two_fingerprints_never_coincide() {
    for config in [COLOUR_CONFIG, "{}", "", "{", "[]", r#"{"a": 1}"#] {
        let (_tmp, p) = project(&[(CONFIG_FILE, config), (ACTIONS_FILE, ACTIONS)]);
        let snapshot = ProjectSnapshot::load(&p);
        assert_ne!(snapshot.fingerprint(), snapshot.legacy_fingerprint(), "{config:?}");
        assert_eq!(snapshot.fingerprint().len(), 64);
    }
}

/// The same for approvals: nothing is trusted, the file is kept, and approving saves again.
#[test]
fn a_broken_trust_file_is_kept_aside() {
    let data = TempDir::new().expect("tempdir");
    std::fs::write(data.path().join(TrustStore::FILE_NAME), "[oops").expect("write");
    let (mut store, err) = TrustStore::load_or_recover(data.path());
    assert!(err.is_some());
    let (_tmp, p) = project(&[(ACTIONS_FILE, ACTIONS)]);
    assert_eq!(store.status(&p), TrustStatus::Untrusted);
    store.trust(&p);
    store.save().expect("save");
    assert_eq!(TrustStore::load(data.path()).expect("load").status(&p), TrustStatus::Trusted);
    let aside = data.path().join(format!("{}.bad", TrustStore::FILE_NAME));
    assert_eq!(std::fs::read_to_string(aside).expect("kept"), "[oops");
}
