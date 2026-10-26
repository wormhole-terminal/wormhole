//! How each terminal compares with Wormhole, as data: one strength and a few drawbacks per terminal,
//! each drawback with the links it was checked against and the day it was checked.
//!
//! The sentences are catalog messages in `wormhole-text` (`settings-import-strength-<source>-<id>`,
//! `settings-import-gap-<source>-<id>`), so this file holds only ids, which stay when a sentence is
//! reworded and go when the fact stops being true. Dates are data (`YYYY-MM-DD`), worded by the catalog. The content is `repo-docs/docs/terminal-comparison-app.md`
//! (checked against primary sources on [`CHECKED`]); an entry marked there as "re-check" is looked at
//! again at each release of that terminal.
//!
//! Wormhole's own list is here too, shown the same way and last. The card states each drawback as a
//! fact about the other terminal, never as "unlike Wormhole".

use super::source::Source;

/// When every entry below was checked against its source.
pub const CHECKED: &str = "2026-10-26";

/// One thing a terminal falls short on today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawback {
    /// Stable: names the catalog message (`settings-import-gap-<terminal>-<id>`).
    pub id: &'static str,
    /// The pages the claim was checked against, each linked in the card. Empty where the source is a
    /// private note that has no public page yet: the card then shows the line without a link.
    pub sources: &'static [&'static str],
    /// The day the claim was checked, `YYYY-MM-DD`: the card says "As of" it.
    pub date: &'static str,
    /// A claim about something that moves (a release, an open request): look at it again at each
    /// release of that terminal. The card does not show this.
    pub recheck: bool,
}

/// What the card says of one terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Names the one-line strength message (`settings-import-strength-<terminal>-<id>`).
    pub strength: &'static str,
    /// Three to five, the ones that matter to someone working with coding agents on a Mac.
    pub drawbacks: &'static [Drawback],
}

const fn gap(
    id: &'static str,
    sources: &'static [&'static str],
    date: &'static str,
    recheck: bool,
) -> Drawback {
    Drawback { id, sources, date, recheck }
}

/// What the card says of `source`.
pub fn of(source: Source) -> &'static Comparison {
    match source {
        Source::Ghostty => &GHOSTTY,
        Source::ITerm2 => &ITERM2,
        Source::TerminalApp => &TERMINAL_APP,
        Source::Alacritty => &ALACRITTY,
        Source::Kitty => &KITTY,
        Source::WezTerm => &WEZTERM,
        Source::Warp => &WARP,
        Source::Zed => &ZED,
        Source::VsCode => &VSCODE,
    }
}

const GHOSTTY: Comparison = Comparison {
    strength: "native_tabs_graphics",
    drawbacks: &[
        gap("no_session_persistence", &["https://ghostty.org/docs/config/reference"], "2026-10-26", false),
        gap(
            "agent_status_not_shown",
            &["https://github.com/ghostty-org/ghostty/pull/14560"],
            "2026-10-06",
            true,
        ),
        gap("command_notify_off", &["https://ghostty.org/docs/config/reference"], "2026-10-26", false),
        gap("no_sixel", &["https://github.com/ghostty-org/ghostty/discussions/2496"], "2026-10-26", false),
        gap(
            "config_file_only",
            &["https://ghostty.org/docs/install/release-notes/1-3-0"],
            "2026-10-26",
            true,
        ),
    ],
};

const ITERM2: Comparison = Comparison {
    strength: "broadest_features",
    drawbacks: &[
        gap("quit_ends_programs", &["https://iterm2.com/restoration.html"], "2026-10-26", false),
        gap(
            "ligatures_disable_gpu",
            &["https://iterm2.com/documentation-one-page.html"],
            "2026-10-26",
            false,
        ),
        gap("ai_setup_required", &["https://iterm2.com/documentation-one-page.html"], "2026-10-26", false),
    ],
};

const TERMINAL_APP: Comparison = Comparison {
    strength: "built_into_macos",
    drawbacks: &[
        gap(
            "no_kitty_keyboard",
            &["https://garrett.damore.org/2025/12/macos-terminal-still-missing-mark-apple.html"],
            "2025-12-25",
            true,
        ),
        gap(
            "no_inline_images",
            &["https://www.arewesixelyet.com/", "https://support.apple.com/guide/terminal/welcome/mac"],
            "2026-10-26",
            false,
        ),
        gap(
            "no_agent_status",
            &["https://support.apple.com/guide/terminal/welcome/mac"],
            "2026-10-26",
            false,
        ),
    ],
};

const ALACRITTY: Comparison = Comparison {
    strength: "minimal_fast",
    drawbacks: &[
        gap("no_tabs_or_splits", &["https://github.com/alacritty/alacritty#faq"], "2026-10-26", false),
        gap("no_images", &["https://github.com/alacritty/alacritty/issues/8891"], "2026-03-29", false),
        gap("no_ligatures", &["https://github.com/alacritty/alacritty/issues/50"], "2026-10-26", false),
        gap("config_file_only", &["https://github.com/alacritty/alacritty#faq"], "2026-10-26", false),
        gap(
            "no_agent_status",
            &["https://github.com/alacritty/alacritty/blob/master/extra/man/alacritty-escapes.7.scd"],
            "2026-10-26",
            false,
        ),
    ],
};

const KITTY: Comparison = Comparison {
    strength: "fast_scriptable",
    drawbacks: &[
        gap(
            "no_session_persistence",
            &["https://sw.kovidgoyal.net/kitty/sessions/", "https://sw.kovidgoyal.net/kitty/faq/"],
            "2026-10-26",
            false,
        ),
        gap("gui_launch_env", &["https://sw.kovidgoyal.net/kitty/faq/"], "2026-10-26", false),
        gap(
            "no_sixel",
            &["https://www.arewesixelyet.com/", "https://github.com/kovidgoyal/kitty/issues/2511"],
            "2026-10-26",
            false,
        ),
        gap("config_file_only", &["https://sw.kovidgoyal.net/kitty/overview/"], "2026-10-26", false),
    ],
};

const WEZTERM: Comparison = Comparison {
    strength: "multiplexer_lua",
    drawbacks: &[
        gap(
            "no_release_since_2024",
            &[
                "https://github.com/wezterm/wezterm/releases",
                "https://github.com/wezterm/wezterm/issues/6341",
            ],
            "2026-10-26",
            true,
        ),
        gap("persistence_needs_setup", &["https://wezterm.org/multiplexing.html"], "2026-10-26", false),
        gap(
            "kitty_keyboard_off",
            &["https://wezterm.org/config/lua/config/enable_kitty_keyboard.html"],
            "2026-10-26",
            false,
        ),
        gap("config_is_lua", &["https://wezterm.org/config/files.html"], "2026-10-26", false),
        gap(
            "macos_26_reports",
            &[
                "https://github.com/wezterm/wezterm/issues/7249",
                "https://github.com/wezterm/wezterm/issues/7271",
            ],
            "2026-10-26",
            true,
        ),
    ],
};

const WARP: Comparison = Comparison {
    strength: "agent_focused",
    drawbacks: &[
        gap(
            "programs_end_on_relaunch",
            &["https://github.com/warpdotdev/warp/issues/9416"],
            "2026-10-26",
            false,
        ),
        gap(
            "telemetry_for_free_ai",
            &["https://docs.warp.dev/support-and-community/privacy-and-security/privacy"],
            "2026-10-26",
            false,
        ),
        gap(
            "login_for_some_features",
            &[
                "https://www.warp.dev/blog/lifting-login-requirement",
                "https://docs.warp.dev/support-and-community/privacy-and-security/privacy",
            ],
            "2026-10-26",
            false,
        ),
        gap("offline_requested", &["https://github.com/warpdotdev/warp/issues/5640"], "2026-10-26", false),
        gap(
            "no_sixel",
            &[
                "https://github.com/warpdotdev/warp/issues/4282",
                "https://github.com/warpdotdev/warp/issues/26",
            ],
            "2026-10-26",
            false,
        ),
    ],
};

const ZED: Comparison = Comparison {
    strength: "terminal_in_editor",
    drawbacks: &[
        gap("editor_panel", &["https://zed.dev/docs/terminal"], "2026-10-26", false),
        gap("no_images", &["https://github.com/zed-industries/zed/issues/20860"], "2026-10-26", false),
        gap(
            "no_kitty_keyboard",
            &["https://github.com/zed-industries/zed/blob/main/crates/terminal/src/mappings/keys.rs"],
            "2026-10-26",
            true,
        ),
        gap(
            "terminal_ligatures",
            &["https://github.com/zed-industries/zed/issues/11127"],
            "2026-10-26",
            false,
        ),
        gap(
            "telemetry_on",
            &[
                "https://github.com/zed-industries/zed/blob/main/assets/settings/default.json",
                "https://zed.dev/docs/telemetry",
            ],
            "2026-10-26",
            false,
        ),
    ],
};

const VSCODE: Comparison = Comparison {
    strength: "terminal_in_editor",
    drawbacks: &[
        gap("editor_panel", &["https://code.visualstudio.com/docs/terminal/basics"], "2026-10-26", false),
        gap(
            "programs_relaunched",
            &["https://code.visualstudio.com/docs/terminal/advanced"],
            "2026-10-26",
            false,
        ),
        gap(
            "images_off",
            &[
                "https://code.visualstudio.com/docs/terminal/advanced",
                "https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/common/terminalConfiguration.ts",
            ],
            "2026-10-26",
            false,
        ),
        gap("scrollback_1000", &["https://code.visualstudio.com/docs/terminal/basics"], "2026-10-26", false),
        gap("telemetry_on", &["https://code.visualstudio.com/docs/configure/telemetry"], "2026-10-26", false),
    ],
};

/// Wormhole's own, shown last under "What Wormhole doesn't do yet". Notes and tasks, project folders
/// and persistent terminals are behind feature flags that are off in a release, so they are neither
/// listed as features nor as gaps.
pub const WORMHOLE: Comparison = Comparison {
    strength: "agent_sidebar",
    drawbacks: &[
        gap("macos_only", &["https://github.com/wormhole-terminal/wormhole#readme"], "2026-10-26", false),
        gap(
            "young_project",
            &["https://github.com/wormhole-terminal/wormhole/blob/main/CHANGELOG.md"],
            "2026-10-07",
            false,
        ),
        gap(
            "programs_end_on_quit",
            &["https://github.com/wormhole-terminal/wormhole#readme"],
            "2026-10-26",
            true,
        ),
        // The next two are in the private limitations notes, which have no public page yet.
        gap("voiceover_no_terminal_text", &[], "2026-10-26", false),
        gap("kitty_keyboard_partial", &[], "2026-10-26", false),
    ],
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn every() -> impl Iterator<Item = (&'static str, &'static Comparison)> {
        Source::ALL.into_iter().map(|source| (source.key(), of(source))).chain([("wormhole", &WORMHOLE)])
    }

    #[test]
    fn every_source_and_wormhole_has_three_to_five_drawbacks() {
        for (name, comparison) in every() {
            let n = comparison.drawbacks.len();
            assert!((3..=5).contains(&n), "{name} has {n}");
            assert!(!comparison.strength.is_empty(), "{name}");
        }
    }

    #[test]
    fn ids_are_unique_per_terminal_and_names_of_messages() {
        for (name, comparison) in every() {
            let mut seen = HashSet::new();
            for drawback in comparison.drawbacks {
                assert!(seen.insert(drawback.id), "{name}: {} twice", drawback.id);
                assert!(
                    drawback.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                    "{name}: {}",
                    drawback.id
                );
                for url in drawback.sources {
                    assert!(url.starts_with("https://") && !url.contains(' '), "{name}: {}", drawback.id);
                }
            }
        }
    }

    #[test]
    fn dates_are_days_and_no_sentence_is_in_the_data() {
        for (name, comparison) in every() {
            for drawback in comparison.drawbacks {
                let day = drawback.date.as_bytes();
                let shaped = day.len() == 10
                    && day
                        .iter()
                        .enumerate()
                        .all(|(i, b)| if i == 4 || i == 7 { *b == b'-' } else { b.is_ascii_digit() });
                assert!(shaped, "{name}: {}: {:?}", drawback.id, drawback.date);
                assert!(drawback.date <= CHECKED, "{name}: {} is dated after the check", drawback.id);
            }
        }
    }

    #[test]
    fn a_drawback_may_have_several_sources_and_a_private_one_none() {
        let several = TERMINAL_APP.drawbacks.iter().find(|d| d.id == "no_inline_images").expect("entry");
        assert_eq!(several.sources.len(), 2);
        let private =
            WORMHOLE.drawbacks.iter().find(|d| d.id == "voiceover_no_terminal_text").expect("entry");
        assert!(private.sources.is_empty());
    }

    #[test]
    fn what_needs_a_second_look_is_marked() {
        assert!(GHOSTTY.drawbacks[1].recheck);
        assert!(!GHOSTTY.drawbacks[0].recheck);
    }
}
