//! The controls of the terminal page the native window writes through: each setting's menu or switch over the settings.

use wormhole_core::config::OptionAsMeta;
use wormhole_core::config::clipboard::{ClipboardRead, ClipboardWrite};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::{Current, MenuOption, Setting, toggle};

/// The scrollback sizes offered; a value set in config.json shows as it is.
const SCROLLBACK: [usize; 6] = [1_000, 5_000, 10_000, 50_000, 100_000, 200_000];

/// "10,000 lines" for a menu option.
fn scrollback_lines(n: u32) -> String {
    lines(n as usize)
}

/// "10,000 lines".
fn lines(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (ix, c) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    settings_terminal_scrollback_lines(&grouped)
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::menu_of_values(
        "scrollback",
        &[
            MenuOption::counted(scrollback_lines, 1_000),
            MenuOption::counted(scrollback_lines, 5_000),
            MenuOption::counted(scrollback_lines, 10_000),
            MenuOption::counted(scrollback_lines, 50_000),
            MenuOption::counted(scrollback_lines, 100_000),
            MenuOption::counted(scrollback_lines, 200_000),
        ],
        |s| match SCROLLBACK.iter().position(|n| *n == s.scrollback_lines) {
            Some(at) => Current::At(at),
            None => Current::Other(lines(s.scrollback_lines)),
        },
        |s, ix| s.scrollback_lines = SCROLLBACK.get(ix).copied().unwrap_or(s.scrollback_lines),
    ),
    toggle!("restore-text", restore_terminal_text),
    toggle!("minimap", minimap),
    toggle!("split-gaps", split_gaps),
    Setting::menu(
        "meta",
        &[
            MenuOption::text(SETTINGS_TERMINAL_OFF),
            MenuOption::text(SETTINGS_TERMINAL_LEFT),
            MenuOption::text(SETTINGS_TERMINAL_RIGHT),
            MenuOption::text(SETTINGS_TERMINAL_BOTH),
        ],
        |s| OptionAsMeta::ALL.iter().position(|o| *o == s.option_as_meta).unwrap_or(0),
        |s, ix| s.option_as_meta = OptionAsMeta::ALL.get(ix).copied().unwrap_or_default(),
    ),
    toggle!("worktree-clone-build-folders", worktree_clone_build_folders),
    toggle!("bell-sound", bell_sound),
    toggle!("bell-visual", bell_visual),
    Setting::menu(
        "clipboard-read",
        &[
            MenuOption::text(SETTINGS_TERMINAL_ASK),
            MenuOption::text(SETTINGS_TERMINAL_ALLOW),
            MenuOption::text(SETTINGS_TERMINAL_DENY),
        ],
        |s| ClipboardRead::ALL.iter().position(|o| *o == s.clipboard_read).unwrap_or(0),
        |s, ix| s.clipboard_read = ClipboardRead::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::menu(
        "clipboard-write",
        &[MenuOption::text(SETTINGS_TERMINAL_ALLOW), MenuOption::text(SETTINGS_TERMINAL_DENY)],
        |s| ClipboardWrite::ALL.iter().position(|o| *o == s.clipboard_write).unwrap_or(0),
        |s, ix| s.clipboard_write = ClipboardWrite::ALL.get(ix).copied().unwrap_or_default(),
    ),
    toggle!("screen-checksum", allow_screen_checksum),
    toggle!("window-ops", allow_window_ops),
    toggle!("title-report", allow_title_report),
];

#[cfg(test)]
mod tests {
    use super::lines;

    #[test]
    fn line_counts_read_with_thousands_separators() {
        assert_eq!(lines(1_000), "1,000 lines");
        assert_eq!(lines(10_000), "10,000 lines");
        assert_eq!(lines(200_000), "200,000 lines");
        assert_eq!(lines(100), "100 lines");
    }
}
