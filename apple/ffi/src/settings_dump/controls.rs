//! Which control each row has. The app crate's `settings_window/registry.rs` holds these as closures over
//! `Settings`; here they are data, so Swift draws the same rows from one table and the dump test fails when
//! a row has none.
//!
//! Kinds (the Swift `ControlSpec`): `toggle`, `segmented` and `select` (`options`), `stepper`, `slider`,
//! `checkboxes`, `font`, `partsList`, `recorder`, `colorToggle`, `themeCards`, `preview` (a scene of the
//! strip above the page), `dynamicSelect` (options the platform lists: apps, shells, icon themes), `link`
//! (to another page), `action` (a button the host performs), `info` (a read-only value) and `panel` (a
//! block of records the page draws from its own data: agents, imports, the profile).

use serde_json::{Value, json};
use wormhole_core::config::{LINE_HEIGHT, OPACITY};
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::view::layout::title_parts::TitlePart;
#[allow(clippy::wildcard_imports, reason = "the keys are generated, one constant per message")]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use super::options;

type Default<'a> = &'a dyn Fn(&str) -> Option<Value>;

pub(super) fn control(id: &str, keys: &[&str], default: Default) -> Option<Value> {
    if let Some(control) = by_id(id, keys) {
        return Some(control);
    }
    let [key] = keys else { return None };
    if let Some(options) = options::of(key) {
        let kind = if options.len() <= 4 { "segmented" } else { "select" };
        let mut control = json!({ "kind": kind, "options": options });
        if let Some(name) = options::motion_default(key) {
            control["default"] = json!(name);
        }
        return Some(control);
    }
    matches!(default(key), Some(Value::Bool(_))).then(|| json!({ "kind": "toggle" }))
}

fn action(name: &str, label: Key, icon: &str) -> Value {
    json!({ "kind": "action", "action": name, "label": text(label), "icon": icon })
}

fn link(section: &str, label: Key) -> Value {
    json!({ "kind": "link", "section": section, "label": text(label) })
}

fn preview(scene: &str) -> Value {
    json!({ "kind": "preview", "scene": scene })
}

fn panel(name: &str) -> Value {
    json!({ "kind": "panel", "panel": name })
}

fn font(slot: FontSlot) -> Value {
    let (min, max) = slot.size_range();
    json!({ "kind": "font", "slot": slot.key(), "min": min, "max": max,
            "inherit": slot.inherit_label() })
}

fn parts() -> Vec<Value> {
    let labels = [
        TITLE_ROW_PART_FOLDER,
        TITLE_ROW_PART_BRANCH,
        TITLE_ROW_PART_BRANCH_STATUS,
        TITLE_ROW_PART_TERMINAL,
        TITLE_ROW_PART_TERMINAL_STATUS,
        TITLE_ROW_PART_SPACE,
        TITLE_ROW_PART_AGENT_GAUGE,
        TITLE_ROW_PART_PORTS,
        TITLE_ROW_PART_PULL_REQUEST,
        TITLE_ROW_PART_OPEN_IN,
        TITLE_ROW_PART_SPLITS,
    ];
    TitlePart::ALL
        .iter()
        .zip(labels)
        .map(|(part, label)| json!({ "value": part, "label": text(label), "hideable": part.hideable() }))
        .collect()
}

/// The rows that are not a plain switch or a fixed list under one key.
fn by_id(id: &str, keys: &[&str]) -> Option<Value> {
    Some(match id {
        "appearance-preview" => preview("window"),
        "font-specimen" => preview("fonts"),
        "sidebar-preview" => preview("sidebar"),
        "changes-preview" => preview("git"),
        "theme" => json!({ "kind": "themeCards" }),
        "primary-color" => json!({ "kind": "colorToggle" }),
        "branch-color" => json!({ "kind": "segmented", "options": options::of("branch_color"),
                                   "customKey": keys[1], "customWhen": "custom" }),
        "opacity" => json!({ "kind": "slider", "min": OPACITY.start(), "max": OPACITY.end(), "step": 0.01,
                              "format": "percent", "ticks": [0.3, 0.65, 1.0] }),
        "line-height" => json!({ "kind": "slider", "min": LINE_HEIGHT.start(), "max": LINE_HEIGHT.end(),
                                  "step": 0.05, "format": "decimal2", "ticks": [1.0, 1.5, 2.0] }),
        "translucent" => json!({ "kind": "checkboxes", "items": [
            { "key": keys[0], "label": text(SETTINGS_APPEARANCE_SETTINGS) },
            { "key": keys[1], "label": text(SETTINGS_APPEARANCE_THEME_STUDIO) },
            { "key": keys[2], "label": text(SETTINGS_APPEARANCE_SESSION_HOSTS) } ] }),
        "title-row" => json!({ "kind": "partsList", "parts": parts() }),
        "icon-theme" => json!({ "kind": "dynamicSelect", "source": "iconThemes" }),
        "icon-themes-folder" => {
            action("openIconThemesFolder", SETTINGS_APPEARANCE_OPEN_FOLDER, "arrow.up.right.square")
        }
        "font-terminal" => font(FontSlot::Terminal),
        "font-code" => font(FontSlot::Code),
        "font-interface" => font(FontSlot::Interface),
        "fonts-folder" => {
            action("openFontsFolder", SETTINGS_FONTS_OPEN_FONTS_FOLDER, "arrow.up.right.square")
        }
        "custom-fonts" => json!({ "kind": "info", "info": "customFonts" }),
        "terminal-font" => link("fonts", SETTINGS_TERMINAL_EDIT_IN_FONTS),
        "open-in" => json!({ "kind": "dynamicSelect", "source": "openIn" }),
        "default-terminal" => action("makeDefaultTerminal", SETTINGS_TERMINAL_MAKE_DEFAULT, "terminal"),
        "default-shell" => json!({ "kind": "dynamicSelect", "source": "shells" }),
        "shells" | "worktree-base" => action("openConfig", SETTINGS_TERMINAL_EDIT_IN_CONFIG_JSON, "doc.text"),
        "agents-notifications" => link("notifications", SETTINGS_AGENTS_OPEN_NOTIFICATIONS),
        "agents-installed" => panel("agentsInstalled"),
        "agents-other" => panel("agentsOther"),
        "agents-how" => panel("agentsHow"),
        "cli-tool" => panel("cliTool"),
        "agent-access" => panel("agentAccess"),
        "agent-access-programs" => panel("agentAccessPrograms"),
        "agent-skills" | "agent-mcp" => panel(id),
        "import-found" => panel("importFound"),
        "notify-permission" | "notify-test" => panel(id),
        "notify-command-done" => json!({ "kind": "toggleStepper", "stepKey": keys[1], "min": 5, "max": 300,
            "steps": [5, 10, 30, 60, 120, 300] }),
        "hotkey" | "jump-waiting-key" => json!({ "kind": "recorder" }),
        "version" => json!({ "kind": "info", "info": "version" }),
        "update-status" | "count-details" => panel(id),
        "open-config" => {
            action("openConfig", SETTINGS_ADVANCED_OPEN, "chevron.left.forwardslash.chevron.right")
        }
        "open-data" => action("openData", SETTINGS_ADVANCED_SHOW_IN_FINDER, "folder"),
        "open-logs" => action("openLogs", SETTINGS_ADVANCED_SHOW_IN_FINDER, "folder"),
        "reset-settings" => action("resetAll", SETTINGS_ADVANCED_RESET_TO_DEFAULTS, "arrow.counterclockwise"),
        "reset-tips" => action("resetTips", SETTINGS_ADVANCED_RESET_TIPS_BUTTON, "lightbulb"),
        "skip-scans" => action("openDeveloperTools", SETTINGS_ADVANCED_OPEN_DEVELOPER_TOOLS, "gearshape"),
        "tab-width" => json!({ "kind": "stepper", "min": 1, "max": 16, "step": 1,
                               "template": settings_git_tab_width_value(1_234_567) }),
        "profile-name" | "profile-start" | "profile-env" | "profile-git" | "profile-logins"
        | "profile-delete" | "projects" | "probes" => panel(id),
        "feature-flags" => panel(id),
        _ => return None,
    })
}

/// A row shown only while other settings hold: all of them, each `key` equal to `equals`.
pub(super) fn visible_when(id: &str) -> Option<Value> {
    Some(match id {
        "sidebar-changes-scope" | "sidebar-changes-remember" => {
            json!([{ "key": "sidebar_changes", "equals": true }])
        }
        "sidebar-changes-plain" | "sidebar-changes-shared" => json!([
            { "key": "sidebar_changes", "equals": true },
            { "key": "sidebar_changes_scope", "equals": "terminal" }]),
        _ => return None,
    })
}
