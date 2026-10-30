//! Which settings a project may change and which only the person's own file sets, and the list of
//! every key a settings file can have.

use super::Settings;

/// Settings a project's `.wormhole/config.json` may change (how its sessions
/// look and behave while one is in front). The app applies only these keys of
/// a project overlay; every other setting is in [`APP_SETTINGS`]. The theme is not
/// among them: a project brings its own as `.wormhole/theme.json`.
pub const PROJECT_SETTINGS: &[&str] = &[
    "primary_color",
    "theme_primary",
    "branch_color",
    "branch_color_custom",
    "font_family",
    "font_size",
    "font_ligatures",
    "line_height",
    "opacity",
    "blur",
    "density",
    "show_groups_as_cards",
    "fade_group_colors",
    "show_group_folders",
    "option_as_meta",
    "scrollback_lines",
    "changes_tree",
    "group_untracked",
    "diff_wrap",
    "diff_split",
    "tab_width",
    "icon_theme",
    "ui_font_family",
    "ui_font_size",
    "code_font_family",
    "code_font_size",
    "sidebar_status_dot",
    "sidebar_changes",
    "sidebar_notes",
    "sidebar_cmd_hints",
    "bell_sound",
    "bell_visual",
];

/// Settings that are about the app as a whole, which only the user's own
/// `config.json` sets: bringing another project's session to the front must not
/// rebind the global hotkey, hide the menu bar item, turn off updates,
/// restyle the settings and Studio windows, silence notifications about
/// other sessions, or show the status bar in one window and not the next. The
/// theme picks are here too: a project's theme is its `theme.json`, one way to
/// theme a project instead of two that could disagree.
pub const APP_SETTINGS: &[&str] = &[
    "theme",
    "theme_mode",
    "theme_light",
    "hidden_themes",
    // How the app moves is the person's own taste, whatever project is in front.
    "motion",
    "hotkey",
    "menu_bar",
    "auto_update",
    // Whether this Mac is counted: the person's, never a cloned repository's.
    "anonymous_count",
    // Which builds the app installs, and whether config.json may turn unfinished features on: a
    // cloned repository must not be able to choose either.
    "update_channel",
    "notifications",
    "settings_translucent",
    "studio_translucent",
    "status_bar",
    "minimap",
    "split_gaps",
    "column_dividers",
    "icon_animations",
    // The person's own habit: what ⌘1–9 reach, whatever project is in front.
    "sidebar_numbers",
    // How the rows' file counts are worked out is one model for the whole app (two windows can hold
    // terminals of one repository), and it keeps what a terminal changed: not a project's to set.
    "sidebar_changes_scope",
    "sidebar_changes_plain",
    "sidebar_changes_shared",
    "sidebar_changes_remember",
    // The window's chrome, one look for every window (where the terminals are listed too); and the
    // programs the person opens folders in.
    "terminals_in",
    "open_in",
    // The glass is a look of the whole window, and a repository must not switch it on.
    "liquid_glass",
    // What the title row shows is the person's own choice. The pull request also runs `gh`, which
    // a repository must not be able to switch on for itself.
    "title_row",
    "notify_needs_you",
    "notify_agent_done",
    "notify_command_done",
    "notify_command_failed",
    "notify_command_after",
    "notify_bell",
    "notify_only_unseen",
    "notify_in_front",
    "notify_sound",
    "notify_group_bursts",
    "dock_badge",
    "dock_bounce",
    "menu_bar_icon",
    "menu_bar_animate",
    "hotkey_action",
    "jump_waiting_key",
    // How the person reads sizes, not something a project should change.
    "size_units",
    // Programs to run are the person's own choice; a project names its shell with `shell`.
    "default_shell",
    "shells",
    // Whether terminals' text is kept on disk: a cloned repository must not be able to turn that
    // on for itself.
    "restore_terminal_text",
    // Who may control Wormhole through the automation API: a cloned repository must not be able to
    // let programs in for itself.
    "agent_access",
    // Whether scripts may type into terminals and open them: a cloned repository must not be
    // able to turn that on for itself.
    "control_write",
    // Whether programs may read the screen back with DECRQCRA: a cloned repository must not be
    // able to turn that on for itself.
    "allow_screen_checksum",
    // Whether programs may move and resize the window: a cloned repository must not be able to turn
    // that on for itself.
    "allow_window_ops",
    // Whether programs may read the window title back, which types it into the shell: a cloned
    // repository must not be able to turn that on for itself.
    "allow_title_report",
    // Whether programs may read or set the clipboard: a cloned repository must not be able to
    // open it for itself.
    "clipboard_read",
    "clipboard_write",
    // Where worktrees go is the person's own folder layout; a repository must not pick it.
    "worktree_base",
    // Which colour each folder wears is the person's too: a repository can tint its own title row
    // (`project_color`), not recolour the folders in everyone's windows.
    "folder_colors",
    // The folders pinned in the Files tab are the person's own list, not a repository's.
    "files_pinned",
    // Which in-progress features are on is the person's own choice (a dev build, or Unstable), never a
    // project's.
    "feature_flags",
    // Which features the person does without is theirs: a repository can't switch a tool off (or on).
    "features",
];

/// Keys of older files that are still read (into the setting that replaced them) but never written:
/// a file keeps them, and they are not "unknown". (`control_read` is not read any more: the Automation
/// feature's `agent_access` replaced it, and without the feature reading had no effect.)
pub(crate) const LEGACY_SETTINGS: &[&str] = &[
    "compact_sidebar",
    "title_branch_status",
    "title_agent_gauge",
    "title_ports",
    "title_pull_request",
    "control_read",
    // The line under the title row went with the blurred lines it marked the edge of.
    "title_border",
];

/// Every key a settings file can have, sorted.
pub(crate) fn keys() -> Vec<String> {
    // Sorted here: whether a `Map` keeps insertion order depends on serde_json's
    // features, which other crates in a build can turn on.
    let mut keys: Vec<String> = match serde_json::to_value(Settings::default()) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    keys.sort();
    keys
}
