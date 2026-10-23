//! The Sessions and Git sections: the sidebar's terminals and rows, and the Changes panel and diff view.

use super::{entry, group, untitled};
use crate::config::FeatureId;
use crate::config::settings_index::Group;
use wormhole_text::keys::*;

pub(super) const SESSIONS: &[Group] = &[
    untitled(&[entry("sidebar-preview", SETTINGS_SESSIONS_SIDEBAR_PREVIEW, &[])
        .with_keywords(SETTINGS_SESSIONS_SIDEBAR_PREVIEW_KEYWORDS)]),
    group(
        SETTINGS_SESSIONS_GROUP_TERMINALS_IN,
        &[entry("terminals-in", SETTINGS_SESSIONS_TERMINALS_IN, &["terminals_in"])
            .with_description(SETTINGS_SESSIONS_TERMINALS_IN_DESCRIPTION)
            .with_keywords(SETTINGS_SESSIONS_TERMINALS_IN_KEYWORDS)],
    ),
    group(
        SETTINGS_SESSIONS_GROUP_TERMINALS_LIST,
        &[
            entry("density", SETTINGS_SESSIONS_DENSITY, &["density"])
                .with_description(SETTINGS_SESSIONS_DENSITY_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_DENSITY_KEYWORDS),
            entry("cards", SETTINGS_SESSIONS_CARDS, &["show_groups_as_cards"])
                .with_description(SETTINGS_SESSIONS_CARDS_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_CARDS_KEYWORDS),
            entry("fade-group-colors", SETTINGS_SESSIONS_FADE_GROUP_COLORS, &["fade_group_colors"])
                .with_description(SETTINGS_SESSIONS_FADE_GROUP_COLORS_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_FADE_GROUP_COLORS_KEYWORDS),
            entry("show-group-folders", SETTINGS_SESSIONS_SHOW_GROUP_FOLDERS, &["show_group_folders"])
                .with_description(SETTINGS_SESSIONS_SHOW_GROUP_FOLDERS_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_SHOW_GROUP_FOLDERS_KEYWORDS),
            entry("sidebar-numbers", SETTINGS_SESSIONS_SIDEBAR_NUMBERS, &["sidebar_numbers"])
                .with_description(SETTINGS_SESSIONS_SIDEBAR_NUMBERS_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_SIDEBAR_NUMBERS_KEYWORDS),
            entry("cmd-hints", SETTINGS_SESSIONS_CMD_HINTS, &["sidebar_cmd_hints"])
                .with_description(SETTINGS_SESSIONS_CMD_HINTS_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_CMD_HINTS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_SESSIONS_GROUP_ROW_DETAILS,
        &[
            entry("sidebar-changes", SETTINGS_SESSIONS_SIDEBAR_CHANGES, &["sidebar_changes"])
                .with_description(SETTINGS_SESSIONS_SIDEBAR_CHANGES_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_SIDEBAR_CHANGES_KEYWORDS),
            entry("sidebar-dot", SETTINGS_SESSIONS_SIDEBAR_DOT, &["sidebar_status_dot"])
                .with_description(SETTINGS_SESSIONS_SIDEBAR_DOT_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_SIDEBAR_DOT_KEYWORDS),
            entry("sidebar-notes", SETTINGS_SESSIONS_SIDEBAR_NOTES, &["sidebar_notes"])
                .with_description(SETTINGS_SESSIONS_SIDEBAR_NOTES_DESCRIPTION)
                .with_keywords(SETTINGS_SESSIONS_SIDEBAR_NOTES_KEYWORDS)
                .of(FeatureId::Notes),
        ],
    ),
];

pub(super) const GIT: &[Group] = &[
    group(
        SETTINGS_SESSIONS_GROUP_CHANGES_PANEL,
        &[
            entry("changes-view", SETTINGS_GIT_CHANGES_VIEW, &["changes_tree"])
                .with_description(SETTINGS_GIT_CHANGES_VIEW_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_CHANGES_VIEW_KEYWORDS),
            entry("group-untracked", SETTINGS_GIT_GROUP_UNTRACKED, &["group_untracked"])
                .with_description(SETTINGS_GIT_GROUP_UNTRACKED_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_GROUP_UNTRACKED_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_SESSIONS_GROUP_DIFF_VIEW,
        &[
            entry("diff-layout", SETTINGS_GIT_DIFF_LAYOUT, &["diff_split"])
                .with_description(SETTINGS_GIT_DIFF_LAYOUT_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_DIFF_LAYOUT_KEYWORDS),
            entry("diff-wrap", SETTINGS_GIT_DIFF_WRAP, &["diff_wrap"])
                .with_description(SETTINGS_GIT_DIFF_WRAP_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_DIFF_WRAP_KEYWORDS),
            entry("tab-width", SETTINGS_GIT_TAB_WIDTH, &["tab_width"])
                .with_description(SETTINGS_GIT_TAB_WIDTH_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_TAB_WIDTH_KEYWORDS),
            entry("size-units", SETTINGS_GIT_SIZE_UNITS, &["size_units"])
                .with_description(SETTINGS_GIT_SIZE_UNITS_DESCRIPTION)
                .with_keywords(SETTINGS_GIT_SIZE_UNITS_KEYWORDS),
        ],
    ),
];
