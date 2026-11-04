//! The Settings window's sentences with a hole in them, as `wormhole-text` writes them, against the
//! strings the window built from pieces before its words moved to the catalogs: byte for byte, so
//! the move changed nothing a person reads, except one case on purpose: a tab width of 1 now reads
//! "1 space" (the old text said "1 spaces"). In `en-XA` the same sentences are bracketed and accented,
//! which shows that none of them is still assembled here.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Language, with_language};

/// Every typed message of the window with sample arguments, and what its Rust `format!` said.
fn sentences() -> Vec<(String, &'static str)> {
    vec![
        (
            settings_advanced_reset_question(1),
            "Reset 1 changed setting to its default? Your terminals, notes and themes are kept.",
        ),
        (
            settings_advanced_reset_question(3),
            "Reset 3 changed settings to their default? Your terminals, notes and themes are kept.",
        ),
        (settings_agents_hooks_failed("Claude Code", "denied"), "Claude Code hooks: denied"),
        (
            settings_agents_hooks_partial("Codex"),
            "Some of Codex's hook events are missing (it may have been updated). Repair adds them back; your own entries stay.",
        ),
        (settings_agents_login_state("Work", "Status hooks on"), "Work profile · Status hooks on in"),
        (settings_agents_others_head(1), "1 more agent Wormhole recognises"),
        (settings_agents_others_head(4), "4 more agents Wormhole recognises"),
        (settings_agents_cli_install_failed("wh", "denied"), "Couldn't install wh: denied"),
        (settings_agents_cli_remove_failed("wh", "denied"), "Couldn't remove wh: denied"),
        (settings_agents_cli_not_installed("wh"), "wh in any terminal. Not installed"),
        (
            settings_agents_cli_installed_at("wh", "~/.local/bin/wh"),
            "wh in any terminal. Installed at ~/.local/bin/wh",
        ),
        (
            settings_agents_cli_foreign_at("wh", "/usr/bin/wh"),
            "wh in any terminal. Another wh is at /usr/bin/wh",
        ),
        (
            settings_agents_cli_install_tooltip("wh"),
            "Puts wh in /usr/local/bin, or in ~/.local/bin when that can't be written",
        ),
        (settings_agents_cli_foreign_tooltip("wh"), "Another wh is in the way; Wormhole never replaces it"),
        (
            settings_agents_cli_foreign_note("wh"),
            "Wormhole never replaces a command it didn't write. Move that wh away to install this one.",
        ),
        (
            settings_agents_cli_stale_note("wh"),
            "The installed wh runs a copy of Wormhole that is somewhere else. Update points it at this one.",
        ),
        (
            settings_agents_cli_path_on("~/.local/bin", "wh"),
            "~/.local/bin is on your shell's PATH: wh works in new terminals.",
        ),
        (
            settings_agents_cli_path_off("~/.local/bin", "wh", "~/.zshrc"),
            "~/.local/bin isn't on your shell's PATH, so new terminals won't find wh. Add this line to ~/.zshrc, then open a new terminal:",
        ),
        (
            settings_agents_cli_path_unknown("wh", "~/.local/bin"),
            "Couldn't read your shell's PATH. If wh isn't found, add ~/.local/bin to it.",
        ),
        (settings_appearance_primary_yours("#7aa2f7"), "Yours · #7aa2f7, in every theme"),
        (settings_appearance_primary_pick("#7aa2f7"), "Pick a colour below · the theme's is #7aa2f7"),
        (
            settings_appearance_primary_theme("#7aa2f7", "Off: a colour of your own"),
            "The theme's · #7aa2f7. Off: a colour of your own",
        ),
        (
            settings_appearance_project_theme_note("Alpha"),
            "This terminal shows Alpha's theme. Your own is kept for everything else.",
        ),
        (settings_appearance_filter_themes_count(1), "Filter 1 theme"),
        (settings_appearance_filter_themes_count(48), "Filter 48 themes"),
        (settings_appearance_show_all_themes("48"), "Show all 48"),
        (settings_fonts_count(1), "1 font"),
        (settings_fonts_count(0), "0 fonts"),
        (settings_fonts_count(12), "12 fonts"),
        (settings_fonts_families_found(1, "Berkeley Mono"), "1 family found: Berkeley Mono"),
        (
            settings_fonts_families_found(2, "Berkeley Mono, Monaspace Neon"),
            "2 families found: Berkeley Mono, Monaspace Neon",
        ),
        (settings_fonts_size_points("14"), "14 pt"),
        (settings_fonts_specimen_size("JetBrains Mono", "14"), "JetBrains Mono 14 pt"),
        (settings_fonts_specimen_line_height("1.30"), "line height 1.30"),
        (settings_fonts_specimen_colours("Soltari"), "Soltari colours"),
        (settings_git_tab_width_value(1), "1 space"),
        (settings_git_tab_width_value(4), "4 spaces"),
        (settings_menu_bar_key_conflict("⌘Space is Spotlight's"), "⌘Space is Spotlight's. Press other keys."),
        (settings_notifications_after_minutes("5"), "5 min"),
        (settings_notifications_after_seconds("30"), "30 s"),
        (
            settings_profile_start_caption_none("Work"),
            "Where a new terminal starts. A folder you add below opens in Work too.",
        ),
        (settings_profile_start_caption_one("~/work", "Work"), "Folders under ~/work open in Work too."),
        (
            settings_profile_start_caption_many("~/work, ~/clients", "~/ops", "Work"),
            "Folders under ~/work, ~/clients and ~/ops open in Work too.",
        ),
        (
            settings_profile_delete_question("Work"),
            "Delete Work? Its terminals, notes, tasks and settings are removed, and its windows close. Wormhole goes to Default.",
        ),
        (settings_profile_env_not_a_pair("FOO"), "“FOO” is not NAME=value"),
        (settings_profile_start_from_copy("Work"), "A copy of Work"),
        (settings_projects_missing_folder("/tmp/alpha"), "/tmp/alpha has no .wormhole folder anymore."),
        (settings_projects_show_in_finder(".wormhole"), "Show .wormhole in Finder"),
        (
            settings_projects_trust_applies("its settings in config.json"),
            "Applies its settings in config.json.",
        ),
        (
            settings_projects_trust_again("its settings in config.json"),
            "Trust it again to apply its settings in config.json.",
        ),
        (
            settings_projects_trust_would_apply("its settings in config.json"),
            "Trusting it applies its settings in config.json.",
        ),
        (
            settings_projects_feature_needs_trust("Per-project settings"),
            "Per-project settings. Only after you trust the project",
        ),
        (settings_sessions_preview_now("Regular"), "Now · Regular"),
        (settings_sessions_preview_hovering("Compact"), "Previewed while hovering Compact"),
        (settings_terminal_default_failed("denied"), "Couldn't make Wormhole the default: denied"),
        (
            settings_terminal_font_summary_ligatures_on("JetBrains Mono", "14", "1.30"),
            "JetBrains Mono 14 pt · line height 1.30 · ligatures on",
        ),
        (
            settings_terminal_font_summary_ligatures_off("JetBrains Mono", "14", "1.30"),
            "JetBrains Mono 14 pt · line height 1.30 · ligatures off",
        ),
        (
            settings_terminal_worktree_base_under("~/worktrees"),
            "Under ~/worktrees, in a folder named after the repository (worktree_base in config.json)",
        ),
        (settings_terminal_scrollback_lines("10,000"), "10,000 lines"),
        (settings_terminal_login_shell("/bin/zsh -l"), "Login shell · /bin/zsh -l"),
        (settings_updates_checked_minutes_ago(2), "Checked 2 minutes ago."),
        (settings_updates_checked_hours_ago(5), "Checked 5 hours ago."),
        (settings_updates_checked_days_ago(3), "Checked 3 days ago."),
        (
            settings_updates_up_to_date_checked("Checked just now."),
            "Wormhole is up to date. Checked just now.",
        ),
        (settings_updates_security_available("0.3.1"), "Security update available: Wormhole 0.3.1."),
        (settings_updates_version_available("0.3.1"), "Version 0.3.1 is available."),
        (
            settings_updates_installed_waits("0.3.1"),
            "Version 0.3.1 is installed. Restarts when your terminals are idle.",
        ),
        (settings_updates_installed_restart("0.3.1"), "Version 0.3.1 is installed. Restart to use it."),
        (settings_updates_whats_new("0.3.1"), "What's new in 0.3.1"),
        (settings_window_open_section("Fonts"), "Open Fonts"),
        (settings_window_import_failed("not a theme"), "Import failed: not a theme"),
        (
            settings_window_results_summary(1, 1, "tab"),
            "1 setting in 1 section match “tab”. Pick a section to see it in context.",
        ),
        (
            settings_window_results_summary(1, 2, "tab"),
            "1 setting in 2 sections match “tab”. Pick a section to see it in context.",
        ),
        (
            settings_window_results_summary(3, 1, "line"),
            "3 settings in 1 section match “line”. Pick a section to see it in context.",
        ),
        (
            settings_window_results_summary(7, 4, "line"),
            "7 settings in 4 sections match “line”. Pick a section to see it in context.",
        ),
        (
            settings_window_preview_feature_note("project_folders"),
            "Preview feature, behind the “project_folders” flag — a release build doesn't have it yet.",
        ),
        (
            settings_window_profiles_unreadable("expected a list"),
            "The list of profiles couldn't be read, so it was set aside and rebuilt: expected a list",
        ),
        (settings_window_nothing_matches("zzz"), "Nothing matches “zzz”"),
        (
            settings_window_set_by_project("Alpha", "alpha", "config.json"),
            "Set by Alpha/alpha/config.json while one of its terminals is in front; yours is kept",
        ),
    ]
}

#[test]
fn the_sentences_read_as_the_code_built_them() {
    for (said, built) in sentences() {
        assert_eq!(said, built);
    }
}

#[test]
fn in_the_pseudo_language_every_sentence_is_a_message() {
    with_language(Language::PSEUDO, || {
        for (said, built) in sentences() {
            assert!(said.starts_with('⟦') && said.ends_with('⟧'), "{built:?} came out as {said:?}");
            assert_ne!(said, built);
        }
    });
}
