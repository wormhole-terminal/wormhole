use std::path::Path;

use super::{
    AgentLogin, DEFAULT_ID, DataRoot, NewProfile, Profile, ProfileError, ProfileId, ProfileList, StartFrom,
    login_env_var,
};
use crate::agents::AgentId;
use crate::config::Config;
use crate::sidebar::GroupColor;
use crate::store::Store;

fn root() -> (tempfile::TempDir, DataRoot) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = DataRoot::new(dir.path());
    (dir, root)
}

fn id(text: &str) -> ProfileId {
    ProfileId::parse(text).expect("a valid id")
}

fn new(name: &str, from: StartFrom) -> NewProfile {
    NewProfile { name: name.into(), color: None, avatar: None, from }
}

#[test]
fn default_keeps_the_data_folders_own_files_and_the_others_get_a_folder() {
    let (dir, root) = root();
    let default = root.default_profile();
    assert_eq!(default.dir(), dir.path(), "an existing user's data does not move");
    assert_eq!(default.config_file(), dir.path().join("config.json"));
    assert_eq!(default.themes_dir(), dir.path().join("themes"));
    let work = root.profile(&id("work"));
    assert_eq!(work.dir(), dir.path().join("profiles/work"));
    assert_eq!(work.config_file(), dir.path().join("profiles/work/config.json"));
    // What every profile shares is at the top.
    assert_eq!(root.update_schedule_file(), dir.path().join("update-schedule.json"));
    assert_eq!(root.profiles_file(), dir.path().join("profiles.json"));
}

#[test]
fn ids_cannot_lead_out_of_the_data_folder() {
    for bad in ["", "..", "a/b", "A", "-x", "x-", "a b", "é", &"a".repeat(33)] {
        assert!(ProfileId::parse(bad).is_err(), "{bad:?}");
    }
    assert!(ProfileId::parse("work-2").is_ok());
    let (_dir, root) = root();
    std::fs::write(
        root.profiles_file(),
        r#"{"profiles":[{"id":"../../etc","name":"Evil"},{"id":"ok","name":"Fine"}]}"#,
    )
    .expect("writes");
    let list = ProfileList::load(&root).expect("loads");
    assert_eq!(list.profiles().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), [DEFAULT_ID, "ok"]);
}

#[test]
fn a_fresh_data_folder_has_only_default_and_writes_nothing() {
    let (_dir, root) = root();
    let list = ProfileList::load(&root).expect("loads");
    assert_eq!(list.profiles().len(), 1);
    assert!(list.active_id().is_default());
    assert!(!root.profiles_file().exists());
}

#[test]
fn the_list_round_trips_and_remembers_the_active_profile() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let work = list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    list.edit(&work, |p| {
        p.start_dir = "~/work".into();
        p.folders = vec!["~/work".into(), "~/clients".into()];
        p.env.insert("AWS_PROFILE".into(), "acme-prod".into());
        p.git.name = "Benny".into();
        p.git.email = "benny@acme.dev".into();
        p.agent_logins = vec![AgentLogin { agent: AgentId::Claude, dir: "~/.claude-work".into() }];
    })
    .expect("edits");
    list.set_active(&work).expect("sets");
    list.save(&root).expect("saves");
    let again = ProfileList::load(&root).expect("loads");
    assert_eq!(again, list);
    assert_eq!(again.active().name, "Work");
    assert_eq!(again.active().avatar, 'W');
}

#[test]
fn ids_come_from_names_and_never_repeat() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let a = list.create(&root, new("Client Work!", StartFrom::Defaults)).expect("creates");
    assert_eq!(a.as_str(), "client-work");
    // A folder a failed delete left behind is not reused either.
    std::fs::create_dir_all(root.profile(&id("private")).dir()).expect("makes");
    let b = list.create(&root, new("Private", StartFrom::Defaults)).expect("creates");
    assert_eq!(b.as_str(), "private-2");
    let c = list.create(&root, new("日本", StartFrom::Defaults)).expect("creates");
    assert_eq!(c.as_str(), "profile");
    let d = list.create(&root, new("Default2", StartFrom::Defaults)).expect("creates");
    assert_eq!(d.as_str(), "default2");
}

#[test]
fn names_are_checked_and_unique() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    for bad in ["", "   ", "Work", "work", "default", &"x".repeat(41)] {
        let before = list.clone();
        assert!(list.create(&root, new(bad, StartFrom::Defaults)).is_err(), "{bad:?}");
        assert_eq!(list, before);
    }
    let other = list.create(&root, new("Private", StartFrom::Defaults)).expect("creates");
    assert!(list.rename(&other, "WORK").is_err());
    list.rename(&other, "  Home ").expect("renames");
    assert_eq!(list.get(&other).map(|p| p.name.as_str()), Some("Home"));
    // Renaming Default is allowed; its id stays.
    list.rename(&ProfileId::default_profile(), "Personal").expect("renames");
    assert_eq!(list.default_profile().id.as_str(), DEFAULT_ID);
}

#[test]
fn colours_go_round_before_repeating() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let mut seen = vec![list.default_profile().color];
    for n in 0..7 {
        let id = list.create(&root, new(&format!("P{n}"), StartFrom::Defaults)).expect("creates");
        seen.push(list.get(&id).expect("listed").color);
    }
    seen.sort_by_key(|c| c.key());
    seen.dedup();
    assert_eq!(seen.len(), 8);
}

#[test]
fn a_copy_takes_settings_themes_and_fonts_but_not_terminals_trust_or_logins() {
    let (_dir, root) = root();
    let default = root.default_profile();
    std::fs::create_dir_all(default.themes_dir()).expect("makes");
    std::fs::write(default.config_file(), r#"{ "font_size": 17 }"#).expect("writes");
    std::fs::write(default.themes_dir().join("mine.json"), "{}").expect("writes");
    std::fs::create_dir_all(default.fonts_dir().join("sub")).expect("makes");
    std::fs::write(default.fonts_dir().join("sub/f.ttf"), b"font").expect("writes");
    std::fs::write(default.dir().join("trusted-projects.json"), "{}").expect("writes");
    let store = Store::open_for(&default).expect("opens");
    store.set_meta("k", "v").expect("sets");
    drop(store);

    let mut list = ProfileList::default();
    list.edit(&ProfileId::default_profile(), |p| {
        p.env.insert("EDITOR".into(), "hx".into());
        p.folders = vec!["~/personal".into()];
        p.agent_logins = vec![AgentLogin { agent: AgentId::Codex, dir: "~/.codex".into() }];
    })
    .expect("edits");
    let work =
        list.create(&root, new("Work", StartFrom::Copy(ProfileId::default_profile()))).expect("creates");
    let paths = root.profile(&work);
    assert_eq!(std::fs::read_to_string(paths.config_file()).expect("reads"), r#"{ "font_size": 17 }"#);
    assert!(paths.themes_dir().join("mine.json").exists());
    assert!(paths.fonts_dir().join("sub/f.ttf").exists());
    assert!(!paths.dir().join("trusted-projects.json").exists());
    assert!(!paths.dir().join("wormhole.db").exists(), "its terminals stay in Default");
    let made = list.get(&work).expect("listed");
    assert_eq!(made.env.get("EDITOR").map(String::as_str), Some("hx"));
    assert!(made.agent_logins.is_empty(), "agent logins are never copied");
    assert!(made.folders.is_empty(), "a folder opens in one profile");
    // The source is untouched.
    assert_eq!(list.default_profile().agent_logins.len(), 1);
}

#[test]
fn a_copy_recreates_a_linked_fonts_folder_as_a_link() {
    let (dir, root) = root();
    let shared = dir.path().join("shared-fonts");
    std::fs::create_dir_all(&shared).expect("makes");
    std::os::unix::fs::symlink(&shared, root.default_profile().fonts_dir()).expect("links");
    let mut list = ProfileList::default();
    let work =
        list.create(&root, new("Work", StartFrom::Copy(ProfileId::default_profile()))).expect("creates");
    let fonts = root.profile(&work).fonts_dir();
    assert_eq!(std::fs::read_link(fonts).expect("a link"), shared);
}

#[test]
fn defaults_start_empty_and_a_missing_source_is_refused() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let id = list.create(&root, new("Blank", StartFrom::Defaults)).expect("creates");
    assert!(!root.profile(&id).config_file().exists());
    assert!(matches!(
        list.create(&root, new("Other", StartFrom::Copy(self::id("ghost")))),
        Err(ProfileError::Unknown(_))
    ));
    assert_eq!(list.profiles().len(), 2);
    assert!(!root.profile(&self::id("other")).dir().exists());
}

#[test]
fn deleting_removes_the_profile_and_its_files_but_never_default() {
    let (_dir, root) = root();
    std::fs::write(root.default_profile().config_file(), "{}").expect("writes");
    let mut list = ProfileList::default();
    let work = list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    Store::open_for(&root.profile(&work)).expect("opens");
    list.set_active(&work).expect("sets");
    assert!(matches!(list.delete(&root, &ProfileId::default_profile()), Err(ProfileError::IsDefault)));
    assert!(root.default_profile().config_file().exists());
    let gone = list.delete(&root, &work).expect("deletes");
    assert_eq!(gone.name, "Work");
    assert!(!root.profile(&work).dir().exists());
    assert!(list.active_id().is_default(), "Wormhole falls back to Default");
    assert!(root.default_profile().config_file().exists());
    assert_eq!(ProfileList::load(&root).expect("loads"), list);
    assert!(matches!(list.delete(&root, &work), Err(ProfileError::Unknown(_))));
}

#[test]
fn each_profile_has_its_own_config_and_database() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let work = list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    let (default_paths, work_paths) = (root.default_profile(), root.profile(&work));
    let mut a = Config::load_for(&default_paths);
    let mut b = Config::load_for(&work_paths);
    assert!(default_paths.config_file().exists() && work_paths.config_file().exists());
    b.update(|s| s.font_size = 19.0);
    a.update(|s| s.font_size = 13.0);
    assert_eq!(Config::load_for(&work_paths).settings.font_size, 19.0);
    assert_eq!(Config::load_for(&default_paths).settings.font_size, 13.0);
    Store::open_for(&default_paths).expect("opens").set_meta("k", "default").expect("sets");
    Store::open_for(&work_paths).expect("opens").set_meta("k", "work").expect("sets");
    assert_eq!(
        Store::open_for(&default_paths).expect("opens").meta("k").expect("reads").as_deref(),
        Some("default")
    );
    assert_eq!(
        Store::open_for(&work_paths).expect("opens").meta("k").expect("reads").as_deref(),
        Some("work")
    );
}

#[test]
fn a_damaged_list_is_kept_aside_and_rebuilt_from_the_folders() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    std::fs::write(root.profiles_file(), "{ not json").expect("writes");
    let (rebuilt, err) = ProfileList::load_or_recover(&root);
    assert!(matches!(err, Some(ProfileError::Parse { .. })));
    assert!(root.base().join("profiles.json.bad").exists());
    assert_eq!(rebuilt.profiles().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), [DEFAULT_ID, "work"]);
}

#[test]
fn one_bad_entry_does_not_take_the_others_and_values_are_cleaned() {
    let (_dir, root) = root();
    std::fs::write(
        root.profiles_file(),
        r#"{ "active": "ghost", "profiles": [
            { "id": "default", "name": "Default" },
            { "id": 5 },
            { "id": "work", "name": "", "color": "chartreuse", "avatar": "toolong",
              "env": { "OK": "1", "WORMHOLE_TAB_ID": "x", "1BAD": "y", "A=B": "z" },
              "folders": ["~/work", "relative", "/a/../b"],
              "agent_logins": [{"agent": "claude", "dir": "~/.c"}, {"agent": "aider", "dir": "~/.a"}],
              "git": { "name": "multi\nline", "email": "a@b.c" } }
        ] }"#,
    )
    .expect("writes");
    let list = ProfileList::load(&root).expect("loads");
    assert!(list.active_id().is_default(), "an unknown active profile is Default");
    let work = list.get(&id("work")).expect("kept");
    assert_eq!(work.name, "work");
    assert_eq!(work.color, GroupColor::Blue);
    assert_eq!(work.avatar, 'W');
    assert_eq!(work.env.keys().collect::<Vec<_>>(), ["OK"]);
    assert_eq!(work.folders, ["~/work"]);
    assert_eq!(work.agent_logins.len(), 1);
    assert_eq!(work.git.name, "");
    assert_eq!(work.git.email, "a@b.c");
    assert!(work.check().is_ok());
}

#[test]
fn edits_that_are_not_allowed_leave_the_profile_alone() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let work = list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    let before = list.clone();
    let bad: [fn(&mut Profile); 7] = [
        |p| {
            p.env.insert("WORMHOLE_SOCKET".into(), "/x".into());
        },
        |p| {
            p.env.insert("A B".into(), "x".into());
        },
        |p| p.start_dir = "work".into(),
        |p| p.folders.push("~/../x".into()),
        |p| p.git.email = "a\nb".into(),
        |p| p.agent_logins.push(AgentLogin { agent: AgentId::Aider, dir: "~/.a".into() }),
        |p| p.avatar = ' ',
    ];
    for change in bad {
        assert!(list.edit(&work, change).is_err());
        assert_eq!(list, before);
    }
    // The id is not the closure's to change.
    list.edit(&work, |p| p.id = id("hijacked")).expect("edits");
    assert!(list.get(&work).is_some());
}

#[test]
fn a_terminal_gets_the_environment_identity_and_logins_of_its_profile() {
    let mut profile = Profile::new(id("work"), "Work", GroupColor::Cyan);
    profile.env.insert("AWS_PROFILE".into(), "acme-prod".into());
    profile.env.insert("GIT_COMMITTER_EMAIL".into(), "ci@acme.dev".into());
    profile.git.name = "Benny".into();
    profile.git.email = "benny@acme.dev".into();
    profile.agent_logins = vec![
        AgentLogin { agent: AgentId::Claude, dir: "~/.claude-work".into() },
        AgentLogin { agent: AgentId::Codex, dir: "/opt/codex".into() },
    ];
    let env = profile.terminal_env_with_home(Some(Path::new("/Users/me")));
    let get = |k: &str| env.get(k).map(String::as_str);
    assert_eq!(get("AWS_PROFILE"), Some("acme-prod"));
    assert_eq!(get("GIT_AUTHOR_NAME"), Some("Benny"));
    assert_eq!(get("GIT_COMMITTER_NAME"), Some("Benny"));
    assert_eq!(get("GIT_AUTHOR_EMAIL"), Some("benny@acme.dev"));
    assert_eq!(get("GIT_COMMITTER_EMAIL"), Some("ci@acme.dev"), "what the person wrote wins");
    assert_eq!(get("CLAUDE_CONFIG_DIR"), Some("/Users/me/.claude-work"));
    assert_eq!(get("CODEX_HOME"), Some("/opt/codex"));
    assert_eq!(login_env_var(AgentId::Aider), None);
    // An empty profile adds nothing: Default's terminals are as they were.
    assert!(Profile::default_profile().terminal_env_with_home(None).is_empty());
}

#[test]
fn a_folder_opens_in_the_profile_that_claims_it_deepest() {
    let (_dir, root) = root();
    let mut list = ProfileList::default();
    let work = list.create(&root, new("Work", StartFrom::Defaults)).expect("creates");
    let acme = list.create(&root, new("Acme", StartFrom::Defaults)).expect("creates");
    list.edit(&work, |p| p.folders = vec!["~/work".into(), "/srv/clients".into()]).expect("edits");
    list.edit(&acme, |p| p.folders = vec!["~/work/acme".into()]).expect("edits");
    let home = Some(Path::new("/Users/me"));
    let of = |path: &str| list.profile_for_folder_with_home(Path::new(path), home).map(|p| p.name.clone());
    assert_eq!(of("/Users/me/work/tool").as_deref(), Some("Work"));
    assert_eq!(of("/Users/me/work/acme/site").as_deref(), Some("Acme"));
    assert_eq!(of("/srv/clients/x").as_deref(), Some("Work"));
    assert_eq!(of("/Users/me/workshop"), None, "a folder, not a name prefix");
    assert_eq!(of("/Users/me/other"), None);
}

#[test]
fn a_profile_saved_with_the_folder_colours_still_loads() {
    let profile: Profile =
        serde_json::from_str(r#"{ "id": "work", "name": "Work", "color": "teal" }"#).expect("loads");
    assert_eq!(profile.color, GroupColor::Cyan, "teal was the old name of cyan");
    let grey: Profile =
        serde_json::from_str(r#"{ "id": "g", "name": "G", "color": "grey" }"#).expect("loads");
    assert_eq!(grey.color, GroupColor::Grey);
}

#[test]
fn the_login_folders_are_resolved_for_the_agents_that_have_one() {
    let mut profile = Profile::new(id("work"), "Work", GroupColor::Cyan);
    profile.agent_logins = vec![
        AgentLogin { agent: AgentId::Claude, dir: "~/.claude-work".into() },
        AgentLogin { agent: AgentId::Aider, dir: "/opt/aider".into() },
    ];
    let home = Path::new("/Users/me");
    assert_eq!(profile.login_dirs(Some(home)), vec![(AgentId::Claude, "/Users/me/.claude-work".into())]);
    assert!(profile.login_dirs(None).is_empty(), "no home, no `~`");
}
