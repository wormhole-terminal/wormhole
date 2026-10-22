//! Every test works in a temp dir standing in for the home directory.

use serde_json::{Value, json};

use super::*;

fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

fn claude() -> &'static Spec {
    spec(AgentId::Claude).expect("claude spec")
}

#[test]
fn claude_install_merges_keeps_a_backup_and_is_idempotent() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    let original = json!({
        "model": "opus",
        "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] }
    });
    write(&path, &serde_json::to_string_pretty(&original).expect("json"));

    assert_eq!(claude().state(home.path()), Ok(State::NotInstalled));
    claude().install(home.path()).expect("install");
    assert_eq!(claude().state(home.path()), Ok(State::Installed));

    let settings = read(&path);
    assert_eq!(settings["model"], "opus");
    let stop = settings["hooks"]["Stop"].as_array().expect("Stop");
    assert_eq!(stop.len(), 2, "the user's hook stays, ours is added");
    assert_eq!(stop[0]["hooks"][0]["command"], "say done");
    assert_eq!(settings["hooks"]["Notification"].as_array().map(Vec::len), Some(1));
    assert_eq!(read(&files::with_suffix(&path, files::BACKUP_SUFFIX)), original);

    let once = std::fs::read_to_string(&path).expect("read");
    claude().install(home.path()).expect("install again");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), once);

    claude().uninstall(home.path()).expect("uninstall");
    assert_eq!(read(&path), original);
    assert_eq!(claude().state(home.path()), Ok(State::NotInstalled));
}

#[test]
fn hooks_installed_by_earlier_versions_are_recognised() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    let legacy = |event: &str| {
        json!([{ "hooks": [{ "type": "command", "timeout": 5, "command":
            format!("[ -n \"$WORMHOLE_BIN\" ] && \"$WORMHOLE_BIN\" hook {event} || true") }] }])
    };
    let mut hooks = serde_json::Map::new();
    for event in json_hooks::CLAUDE.events {
        hooks.insert(event.to_string(), legacy(event));
    }
    write(&path, &json!({ "hooks": hooks }).to_string());
    // Recognised as ours; the status line those versions didn't install yet is what's missing.
    assert_eq!(claude().state(home.path()), Ok(State::Partial));
    claude().uninstall(home.path()).expect("uninstall");
    assert_eq!(read(&path), json!({}));
}

#[test]
fn missing_files_are_created_and_partial_installs_completed() {
    let home = home();
    let gemini = spec(AgentId::Gemini).expect("gemini");
    let path = home.path().join(".gemini/settings.json");
    assert_eq!(gemini.uninstall(home.path()), Ok(()));
    assert!(!path.exists(), "uninstalling nothing writes nothing");

    gemini.install(home.path()).expect("install");
    let mut settings = read(&path);
    let entry = &settings["hooks"]["AfterAgent"][0]["hooks"][0];
    assert_eq!(entry["timeout"], 5000, "Gemini timeouts are milliseconds");
    assert!(entry["command"].as_str().is_some_and(|c| c.contains("--agent gemini AfterAgent")));

    settings["hooks"].as_object_mut().expect("hooks").remove("SessionEnd");
    write(&path, &settings.to_string());
    assert_eq!(gemini.state(home.path()), Ok(State::Partial));
    gemini.install(home.path()).expect("complete");
    assert_eq!(gemini.state(home.path()), Ok(State::Installed));
}

#[test]
fn files_we_cannot_rewrite_faithfully_are_left_alone() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    let commented = "{\n  // my settings\n  \"model\": \"opus\"\n}\n";
    write(&path, commented);
    assert!(claude().install(home.path()).is_err_and(|e| e.contains("comments")));
    write(&path, "[1, 2]");
    assert!(claude().install(home.path()).is_err());
    write(&path, "{ not json");
    assert!(claude().install(home.path()).is_err());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "{ not json");
}

#[test]
fn droid_events_are_top_level_keys() {
    let home = home();
    let droid = spec(AgentId::Droid).expect("droid");
    let path = home.path().join(".factory/hooks.json");
    write(
        &path,
        r#"{"PreToolUse": [{"matcher": "Execute", "hooks": [{"type": "command", "command": "audit"}]}]}"#,
    );
    droid.install(home.path()).expect("install");
    let settings = read(&path);
    assert!(settings.get("hooks").is_none());
    assert_eq!(settings["Stop"].as_array().map(Vec::len), Some(1));
    droid.uninstall(home.path()).expect("uninstall");
    assert_eq!(
        read(&path).as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
        Some(vec!["PreToolUse".into()])
    );
}

#[test]
fn qwen_hooks_print_a_json_object() {
    assert!(hook_command(AgentId::Qwen, "Stop").ends_with("; printf '{}'"));
    assert!(is_our_command(&hook_command(AgentId::Qwen, "Stop")));
    assert!(!hook_command(AgentId::Codex, "Stop").contains("printf"));
}

#[test]
fn own_files_are_added_and_removed_without_touching_others() {
    let home = home();
    let goose = spec(AgentId::Goose).expect("goose");
    let copilot = spec(AgentId::Copilot).expect("copilot");
    let opencode = spec(AgentId::OpenCode).expect("opencode");
    let theirs = home.path().join(".copilot/hooks/theirs.json");
    write(&theirs, "{}");

    for spec in [goose, copilot, opencode] {
        assert_eq!(spec.state(home.path()), Ok(State::NotInstalled));
        spec.install(home.path()).expect("install");
        spec.install(home.path()).expect("install again");
        assert_eq!(spec.state(home.path()), Ok(State::Installed));
    }
    let hooks = read(&home.path().join(".copilot/hooks/wormhole.json"));
    assert_eq!(hooks["version"], 1);
    assert!(hooks["hooks"]["agentStop"][0]["bash"].as_str().is_some_and(is_our_command));
    let plugin =
        std::fs::read_to_string(home.path().join(".config/opencode/plugins/wormhole.js")).expect("plugin");
    assert!(plugin.contains("WORMHOLE_BIN") && plugin.contains("session.idle"));
    assert_eq!(read(&home.path().join(".agents/plugins/wormhole/plugin.json"))["name"], "wormhole");

    for spec in [goose, copilot, opencode] {
        spec.uninstall(home.path()).expect("uninstall");
        assert_eq!(spec.state(home.path()), Ok(State::NotInstalled));
    }
    assert!(theirs.exists());
    assert!(!home.path().join(".agents/plugins/wormhole").exists());
    assert!(home.path().join(".copilot/hooks").exists());
}

#[test]
fn a_foreign_file_in_our_place_blocks_the_install() {
    let home = home();
    let path = home.path().join(".config/opencode/plugins/wormhole.js");
    write(&path, "export const Mine = async () => ({});\n");
    let opencode = spec(AgentId::OpenCode).expect("opencode");
    assert!(opencode.install(home.path()).is_err());
    opencode.uninstall(home.path()).expect("uninstall");
    assert!(path.exists(), "someone else's file is never removed");
}

#[test]
fn events_map_to_statuses() {
    use Signal::*;
    assert_eq!(signal(AgentId::Claude, "Stop"), Some(Done));
    assert_eq!(signal(AgentId::Claude, "Notification:permission_prompt"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "Notification:elicitation_dialog"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "Notification"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "Notification:idle_prompt"), None);
    assert_eq!(signal(AgentId::Claude, "Notification:auth_success"), None);
    assert_eq!(signal(AgentId::Claude, "Notification:agent_completed"), None);
    assert_eq!(signal(AgentId::Claude, "PermissionRequest:Bash"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "PreToolUse:AskUserQuestion"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "PreToolUse:ExitPlanMode"), Some(Attention));
    assert_eq!(signal(AgentId::Claude, "PreToolUse:Bash"), Some(Busy));
    assert_eq!(signal(AgentId::Claude, "Stop:background"), Some(Busy));
    assert_eq!(signal(AgentId::Claude, "StopFailure:rate_limit"), Some(Failed));
    assert_eq!(signal(AgentId::Claude, "StopFailure"), Some(Failed));
    assert_eq!(signal(AgentId::Claude, "SessionEnd:clear"), Some(Quiet));
    assert_eq!(signal(AgentId::Claude, "SessionEnd:prompt_input_exit"), Some(Ended));
    assert_eq!(signal(AgentId::Codex, "PermissionRequest"), Some(Attention));
    assert_eq!(signal(AgentId::Gemini, "AfterAgent"), Some(Done));
    assert_eq!(signal(AgentId::Copilot, "notification:agent_idle"), None);
    assert_eq!(signal(AgentId::Copilot, "notification:permission_prompt"), Some(Attention));
    assert_eq!(signal(AgentId::OpenCode, "session.busy"), Some(Busy));
    assert_eq!(signal(AgentId::Goose, "SessionEnd"), Some(Ended));
    assert_eq!(signal(AgentId::Aider, "Stop"), None);
}

#[test]
fn answering_a_prompt_works_on_and_turning_it_down_stops() {
    use Signal::*;
    assert_eq!(answer_key("enter", false), Some(Busy));
    assert_eq!(answer_key("2", false), Some(Busy));
    assert_eq!(answer_key("escape", false), Some(Quiet));
    assert_eq!(answer_key("c", true), Some(Quiet));
    assert_eq!(answer_key("down", false), None);
    assert_eq!(answer_key("tab", false), None);
    assert_eq!(answer_key("2", true), None);
}

#[test]
fn every_installed_event_means_something() {
    for spec in SPECS {
        let Kind::Json(_, format) = &spec.kind else { continue };
        for event in format.events {
            if spec.agent == AgentId::Gemini && *event == "AfterModel" {
                // This event carries usage only: it must not turn a finished terminal busy.
                assert_eq!(signal(spec.agent, event), None);
                let input = serde_json::json!({"llm_response":{"usageMetadata":{"totalTokenCount":42}}});
                let usage = crate::agents::usage::from_hook(
                    spec.agent,
                    event,
                    &input,
                    std::path::Path::new("/unused"),
                    0,
                )
                .expect("the installed telemetry hook has a consumer");
                assert_eq!(usage.response_tokens, Some(42));
            } else {
                assert!(signal(spec.agent, event).is_some(), "{:?} {event}", spec.agent);
            }
        }
    }
    for (agent, events) in [
        (AgentId::Copilot, &["userPromptSubmitted", "postToolUse", "agentStop", "sessionEnd"][..]),
        (AgentId::Goose, &["UserPromptSubmit", "PostToolUse", "Stop", "SessionEnd"][..]),
        (AgentId::OpenCode, &["session.busy", "session.idle", "permission.asked", "permission.replied"][..]),
    ] {
        for event in events {
            assert!(signal(agent, event).is_some(), "{agent:?} {event}");
        }
    }
}

#[test]
fn install_keeps_the_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    // Claude's settings.json often carries API keys in `env`; a 0600 file must stay 0600.
    let home = home();
    let path = home.path().join(".claude/settings.json");
    write(&path, r#"{"env": {"ANTHROPIC_API_KEY": "secret"}}"#);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    claude().install(home.path()).expect("install");
    let mode = |p: &Path| std::fs::metadata(p).expect("stat").permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&files::with_suffix(&path, files::BACKUP_SUFFIX)), 0o600);
    claude().uninstall(home.path()).expect("uninstall");
    assert_eq!(mode(&path), 0o600);
}

#[test]
fn install_writes_through_a_symlinked_config() {
    // A dotfiles setup: ~/.claude/settings.json links into a repository.
    let home = home();
    let real = home.path().join("dotfiles/claude-settings.json");
    write(&real, r#"{"model": "opus"}"#);
    let link = home.path().join(".claude/settings.json");
    std::fs::create_dir_all(link.parent().expect("parent")).expect("mkdir");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    claude().install(home.path()).expect("install");
    assert!(std::fs::symlink_metadata(&link).expect("lstat").file_type().is_symlink());
    let target = std::fs::read_to_string(&real).expect("read");
    assert!(target.contains("WORMHOLE_BIN"), "the dotfiles copy gets the hooks: {target}");
    assert_eq!(claude().state(home.path()), Ok(State::Installed));
    assert!(files::with_suffix(&link, files::BACKUP_SUFFIX).exists());

    claude().uninstall(home.path()).expect("uninstall");
    assert!(std::fs::symlink_metadata(&link).expect("lstat").file_type().is_symlink());
    assert_eq!(read(&real), json!({ "model": "opus" }));
}

#[test]
fn own_files_are_written_without_temp_leftovers() {
    let home = home();
    let copilot = spec(AgentId::Copilot).expect("copilot");
    copilot.install(home.path()).expect("install");
    let dir = home.path().join(".copilot/hooks");
    let names: Vec<String> = std::fs::read_dir(&dir)
        .expect("list")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["wormhole.json"]);
}

#[test]
fn install_keeps_the_users_key_order() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    write(&path, r#"{"zeta": 1, "alpha": {"z": 1, "a": 2}, "model": "opus"}"#);
    claude().install(home.path()).expect("install");
    let text = std::fs::read_to_string(&path).expect("read");
    let at = |key: &str| text.find(key).unwrap_or_else(|| panic!("{key} missing"));
    assert!(at("\"zeta\"") < at("\"alpha\"") && at("\"alpha\"") < at("\"model\""), "{text}");
    assert!(at("\"z\"") < at("\"a\""), "{text}");
    // Removing our hooks again leaves the user's keys where they were.
    claude().uninstall(home.path()).expect("uninstall");
    let text = std::fs::read_to_string(&path).expect("read");
    let keys: Vec<&str> =
        ["\"zeta\"", "\"alpha\"", "\"model\""].into_iter().filter(|k| text.contains(k)).collect();
    assert_eq!(keys.len(), 3, "{text}");
    let at = |key: &str| text.find(key).unwrap_or_else(|| panic!("{key} missing"));
    assert!(at("\"zeta\"") < at("\"alpha\"") && at("\"alpha\"") < at("\"model\""), "{text}");
}

#[test]
fn uninstall_finds_our_entries_under_any_event() {
    // An event a later version no longer installs (or the user moved our entry).
    let home = home();
    let path = home.path().join(".claude/settings.json");
    claude().install(home.path()).expect("install");
    let mut settings = read(&path);
    let ours = settings["hooks"]["Stop"][0].clone();
    settings["hooks"]["RetiredEvent"] =
        json!([ours, { "hooks": [{ "type": "command", "command": "mine" }] }]);
    write(&path, &settings.to_string());
    claude().uninstall(home.path()).expect("uninstall");
    assert_eq!(
        read(&path),
        json!({ "hooks": { "RetiredEvent": [{ "hooks": [{ "type": "command", "command": "mine" }] }] } })
    );
}

/// `text` as a version up to 0.2 wrote it, when the app went by its old codename.
fn as_before_the_rename(text: &str) -> String {
    text.replace("WORMHOLE", "ANDROMEDA").replace("wormhole", "andromeda").replace("Wormhole", "Andromeda")
}

#[test]
fn hooks_from_before_the_rename_are_replaced_and_the_users_own_kept() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    claude().install(home.path()).expect("install");
    // The same settings as 0.2 left them, around a hook and a status line of the user's.
    let mut old = read(&path);
    old["hooks"]["Stop"]
        .as_array_mut()
        .expect("Stop")
        .insert(0, json!({ "hooks": [{ "type": "command", "command": "say done" }] }));
    old["statusLine"]["command"] = json!(
        "input=$(cat); [ -n \"$ANDROMEDA_BIN\" ] && printf '%s' \"$input\" | \"$ANDROMEDA_BIN\" statusline >/dev/null 2>&1 & printf '%s' \"$input\" | eval '~/.claude/line.sh'"
    );
    write(&path, &as_before_the_rename(&serde_json::to_string_pretty(&old).expect("json")));
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("ANDROMEDA_BIN") && !text.contains("WORMHOLE_BIN"), "the fixture is the old form");

    assert!(claude().has_legacy(home.path()));
    assert_eq!(claude().state(home.path()), Ok(State::Partial), "due to be replaced");
    let upgraded = upgrade_legacy(home.path());
    assert_eq!(upgraded, vec![(AgentId::Claude, Ok(()))], "only the agent that had old hooks");

    let text = std::fs::read_to_string(&path).expect("read");
    assert!(!text.contains("ANDROMEDA"), "nothing of the old names is left:\n{text}");
    assert_eq!(claude().state(home.path()), Ok(State::Installed));
    let settings = read(&path);
    let stop = settings["hooks"]["Stop"].as_array().expect("Stop");
    assert_eq!(stop.len(), 2, "the user's hook and ours, once");
    assert_eq!(stop[0]["hooks"][0]["command"], "say done");
    let line = settings["statusLine"]["command"].as_str().expect("status line");
    assert!(
        line.contains("\"$WORMHOLE_BIN\" statusline") && line.ends_with("eval '~/.claude/line.sh'"),
        "{line}"
    );
    assert!(upgrade_legacy(home.path()).is_empty(), "once");
}

#[test]
fn own_files_from_before_the_rename_are_replaced_and_others_left() {
    let home = home();
    let specs = [AgentId::Copilot, AgentId::Goose, AgentId::OpenCode].map(|a| spec(a).expect("spec"));
    // Each file where 0.2 put it, with what 0.2 wrote; and a file of the user's beside one.
    for spec in specs {
        let Kind::Files(files) = &spec.kind else { panic!("own files") };
        for (rel, text) in files() {
            write(&home.path().join(as_before_the_rename(rel)), &as_before_the_rename(&text));
        }
    }
    let theirs = home.path().join(".config/opencode/plugins/theirs.js");
    write(&theirs, "export const Mine = async () => ({});");

    for spec in specs {
        assert!(spec.has_legacy(home.path()), "{:?}", spec.agent);
        assert_eq!(spec.state(home.path()), Ok(State::Partial));
    }
    let upgraded = upgrade_legacy(home.path());
    assert_eq!(upgraded.len(), 3, "{upgraded:?}");
    assert!(upgraded.iter().all(|(_, result)| result.is_ok()), "{upgraded:?}");
    for spec in specs {
        assert_eq!(spec.state(home.path()), Ok(State::Installed), "{:?}", spec.agent);
        assert!(!spec.has_legacy(home.path()));
    }
    assert!(!home.path().join(".copilot/hooks/andromeda.json").exists());
    assert!(!home.path().join(".config/opencode/plugins/andromeda.js").exists());
    assert!(!home.path().join(".agents/plugins/andromeda").exists(), "the old folder goes");
    assert!(home.path().join(".agents/plugins/wormhole/plugin.json").exists());
    assert!(theirs.exists(), "a file of the user's stays");
}

#[test]
fn uninstalling_takes_the_old_hooks_too() {
    let home = home();
    let path = home.path().join(".claude/settings.json");
    claude().install(home.path()).expect("install");
    let text = std::fs::read_to_string(&path).expect("read");
    write(&path, &as_before_the_rename(&text));
    claude().uninstall(home.path()).expect("uninstall");
    assert_eq!(read(&path), json!({}));
}

#[test]
fn a_login_folder_gets_the_hooks_in_its_own_files_and_the_home_stays_untouched() {
    let home = home();
    let login = home.path().join(".claude-work");
    assert_eq!(claude().path_in(&login), Some(login.join("settings.json")));
    let codex = spec(AgentId::Codex).expect("codex spec");
    assert_eq!(codex.path_in(&login), Some(login.join("hooks.json")));
    assert_eq!(spec(AgentId::Gemini).expect("gemini spec").path_in(&login), None);

    assert_eq!(claude().state_in(&login), Ok(State::NotInstalled));
    claude().install_in(&login).expect("install");
    assert_eq!(claude().state_in(&login), Ok(State::Installed));
    assert_eq!(read(&login.join("settings.json"))["hooks"]["Stop"].as_array().map(Vec::len), Some(1));
    assert_eq!(claude().state(home.path()), Ok(State::NotInstalled), "the default location is not touched");

    claude().uninstall_in(&login).expect("uninstall");
    assert_eq!(claude().state_in(&login), Ok(State::NotInstalled));
    assert!(
        claude().install_in(&login).is_ok()
            && spec(AgentId::Gemini).expect("gemini").install_in(&login).is_err()
    );
}

#[test]
fn gemini_usage_hook_is_added_to_existing_status_hooks() {
    let home = tempfile::tempdir().unwrap();
    let spec = super::spec(super::AgentId::Gemini).unwrap();
    spec.install(home.path()).unwrap();
    let path = spec.path(home.path());
    let mut settings: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    settings["hooks"].as_object_mut().unwrap().remove("AfterModel");
    std::fs::write(&path, serde_json::to_vec(&settings).unwrap()).unwrap();
    assert_eq!(spec.state(home.path()).unwrap(), super::State::Partial);
    spec.install(home.path()).unwrap();
    let settings: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let command = settings["hooks"]["AfterModel"][0]["hooks"][0]["command"].as_str().unwrap();
    assert!(command.contains("gemini") && command.contains("AfterModel"));
    assert_eq!(spec.state(home.path()).unwrap(), super::State::Installed);
}
