//! The Terminal section: scrollback and restoring, opening, the shell, worktrees, the bell and privacy.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const TERMINAL: &[Group] = &[
    group(
        SETTINGS_TERMINAL_GROUP_BEHAVIOUR,
        &[
            entry("scrollback", SETTINGS_TERMINAL_SCROLLBACK, &["scrollback_lines"])
                .with_description(SETTINGS_TERMINAL_SCROLLBACK_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_SCROLLBACK_KEYWORDS),
            entry("restore-text", SETTINGS_TERMINAL_RESTORE_TEXT, &["restore_terminal_text"])
                .with_description(SETTINGS_TERMINAL_RESTORE_TEXT_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_RESTORE_TEXT_KEYWORDS),
            entry("minimap", SETTINGS_TERMINAL_MINIMAP, &["minimap"])
                .with_description(SETTINGS_TERMINAL_MINIMAP_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_MINIMAP_KEYWORDS),
            entry("split-gaps", SETTINGS_TERMINAL_SPLIT_GAPS, &["split_gaps"])
                .with_description(SETTINGS_TERMINAL_SPLIT_GAPS_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_SPLIT_GAPS_KEYWORDS),
            entry("meta", SETTINGS_TERMINAL_META, &["option_as_meta"])
                .with_description(SETTINGS_TERMINAL_META_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_META_KEYWORDS),
            entry("terminal-font", SETTINGS_TERMINAL_FONT, &[])
                .with_keywords(SETTINGS_TERMINAL_FONT_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_TERMINAL_GROUP_OPENING,
        &[
            entry("open-in", SETTINGS_TERMINAL_OPEN_IN, &["open_in"])
                .with_description(SETTINGS_TERMINAL_OPEN_IN_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_OPEN_IN_KEYWORDS),
            entry("default-terminal", SETTINGS_TERMINAL_DEFAULT_TERMINAL, &[])
                .with_description(SETTINGS_TERMINAL_DEFAULT_TERMINAL_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_DEFAULT_TERMINAL_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_TERMINAL_GROUP_SHELL,
        &[
            entry("default-shell", SETTINGS_TERMINAL_DEFAULT_SHELL, &["default_shell"])
                .with_description(SETTINGS_TERMINAL_DEFAULT_SHELL_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_DEFAULT_SHELL_KEYWORDS),
            entry("shells", SETTINGS_TERMINAL_SHELLS, &["shells"])
                .with_description(SETTINGS_TERMINAL_SHELLS_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_SHELLS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_TERMINAL_GROUP_WORKTREES,
        &[
            entry("worktree-base", SETTINGS_TERMINAL_WORKTREE_BASE, &["worktree_base"])
                .with_description(SETTINGS_TERMINAL_WORKTREE_BASE_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_WORKTREE_BASE_KEYWORDS),
            entry(
                "worktree-clone-build-folders",
                SETTINGS_TERMINAL_WORKTREE_CLONE_BUILD_FOLDERS,
                &["worktree_clone_build_folders"],
            )
            .with_description(SETTINGS_TERMINAL_WORKTREE_CLONE_BUILD_FOLDERS_DESCRIPTION)
            .with_keywords(SETTINGS_TERMINAL_WORKTREE_CLONE_BUILD_FOLDERS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_TERMINAL_GROUP_BELL,
        &[
            entry("bell-sound", SETTINGS_TERMINAL_BELL_SOUND, &["bell_sound"])
                .with_description(SETTINGS_TERMINAL_BELL_SOUND_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_BELL_SOUND_KEYWORDS),
            entry("bell-visual", SETTINGS_TERMINAL_BELL_VISUAL, &["bell_visual"])
                .with_description(SETTINGS_TERMINAL_BELL_VISUAL_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_BELL_VISUAL_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_TERMINAL_GROUP_PRIVACY,
        &[
            entry("clipboard-read", SETTINGS_TERMINAL_CLIPBOARD_READ, &["clipboard_read"])
                .with_description(SETTINGS_TERMINAL_CLIPBOARD_READ_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_CLIPBOARD_READ_KEYWORDS),
            entry("clipboard-write", SETTINGS_TERMINAL_CLIPBOARD_WRITE, &["clipboard_write"])
                .with_description(SETTINGS_TERMINAL_CLIPBOARD_WRITE_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_CLIPBOARD_WRITE_KEYWORDS),
            entry("screen-checksum", SETTINGS_TERMINAL_SCREEN_CHECKSUM, &["allow_screen_checksum"])
                .with_description(SETTINGS_TERMINAL_SCREEN_CHECKSUM_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_SCREEN_CHECKSUM_KEYWORDS),
            entry("window-ops", SETTINGS_TERMINAL_WINDOW_OPS, &["allow_window_ops"])
                .with_description(SETTINGS_TERMINAL_WINDOW_OPS_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_WINDOW_OPS_KEYWORDS),
            entry("title-report", SETTINGS_TERMINAL_TITLE_REPORT, &["allow_title_report"])
                .with_description(SETTINGS_TERMINAL_TITLE_REPORT_DESCRIPTION)
                .with_keywords(SETTINGS_TERMINAL_TITLE_REPORT_KEYWORDS),
        ],
    ),
];
