//! The Terminal section: scrollback and restoring, opening, the shell, worktrees, the bell and privacy.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const TERMINAL: &[Group] = &[
    group(
        "Behaviour",
        &[
            entry(
                "scrollback",
                "Scrollback",
                "Lines kept per terminal (applies to new terminals)",
                "history buffer lines",
                &["scrollback_lines"],
            ),
            entry(
                "restore-text",
                "Restore terminal text after restart",
                "Encrypted and kept only on this Mac. Off erases what's saved",
                "scrollback history saved restore privacy secrets encrypted keychain",
                &["restore_terminal_text"],
            ),
            entry(
                "minimap",
                "Show minimap",
                "The scrollback drawn small along the terminal's right edge. Click or drag it to \
                 scroll",
                "overview scrollbar map outline",
                &["minimap"],
            ),
            entry(
                "split-gaps",
                "Gaps between split terminals",
                "Each terminal of a split is its own rounded surface, a small gap apart. Off joins \
                 them with a hairline",
                "panes divider spacing rounded margin",
                &["split_gaps"],
            ),
            entry(
                "meta",
                "Use Option as Meta",
                "For emacs and readline shortcuts. A side set to Meta can't type @ { [ on some \
                 keyboard layouts",
                "alt option escape keyboard left right",
                &["option_as_meta"],
            ),
            entry("terminal-font", "Font", "", "typeface size line height", &[]),
        ],
    ),
    group(
        "Opening",
        &[
            entry(
                "open-in",
                "Open folders in",
                "The Open in… button in a terminal's title row opens its folder here; its menu lists \
                 the others",
                "editor ide zed code vscode cursor sublime xcode jetbrains phpstorm finder app",
                &["open_in"],
            ),
            entry(
                "default-terminal",
                "Default terminal",
                "Double-clicked .command files and programs run in a new Wormhole terminal instead \
                 of Terminal",
                "default terminal app command script executable finder open with launch services",
                &[],
            ),
        ],
    ),
    group(
        "Shell",
        &[
            entry(
                "default-shell",
                "Default shell",
                "What a new terminal runs: your login shell, a shell on this Mac, or a profile \
                 below. Right-click a + for the others",
                "shell zsh bash fish nu nushell login program default",
                &["default_shell"],
            ),
            entry(
                "shells",
                "Shells",
                "Profiles for New Terminal With: a program with its arguments, listed \
                 in config.json",
                "shell profiles custom command arguments args claude program",
                &["shells"],
            ),
        ],
    ),
    group(
        "Worktrees",
        &[entry(
            "worktree-base",
            "Worktrees folder",
            "Where Shell → New Terminal in New Worktree puts a worktree: beside the repository in \
             <name>-worktrees, or under the folder worktree_base names in config.json",
            "worktree worktrees git branch folder base parallel agents",
            &["worktree_base"],
        )],
    ),
    group(
        "Bell",
        &[
            entry(
                "bell-sound",
                "Play a sound",
                "The system beep when a program in a terminal rings the bell",
                "bell beep audible alert",
                &["bell_sound"],
            ),
            entry(
                "bell-visual",
                "Flash the window edge",
                "A short glow along the terminal's edge when it rings the bell",
                "bell visual flash glow",
                &["bell_visual"],
            ),
        ],
    ),
    group(
        "Privacy",
        &[
            entry(
                "clipboard-read",
                "Programs reading the clipboard",
                "What a program gets when it asks for the clipboard's text (OSC 52), over ssh too. Ask \
                 puts a question over that terminal; the clipboard often holds a password",
                "osc 52 clipboard read paste copy password ask allow deny security ssh",
                &["clipboard_read"],
            ),
            entry(
                "clipboard-write",
                "Programs setting the clipboard",
                "Lets a program put text on the clipboard (OSC 52), which replaces what you copied. \
                 The terminal says when one does",
                "osc 52 clipboard write copy set security ssh notice",
                &["clipboard_write"],
            ),
            entry(
                "screen-checksum",
                "Let programs read the screen back",
                "Answers a program's request for the checksum of part of the screen (DECRQCRA), which \
                 also lets a program, over ssh too, read the screen cell by cell. Applies to new \
                 terminals",
                "decrqcra checksum screen read capture esctest security ssh",
                &["allow_screen_checksum"],
            ),
            entry(
                "window-ops",
                "Let programs control the window",
                "Lets a program move, resize, minimise, raise or full-screen the window (xterm's \
                 window operations). Applies to new terminals",
                "xtwinops window operations resize move iconify fullscreen maximize security ssh",
                &["allow_window_ops"],
            ),
            entry(
                "title-report",
                "Let programs read the window title back",
                "Answers a program's request for the window title and icon name (XTWINOPS 20 and \
                 21). A title is text a program chose, and the answer is typed into the shell, so over \
                 ssh this lets a program type a command line. Applies to new terminals",
                "xtwinops title icon name report read back window security ssh esctest",
                &["allow_title_report"],
            ),
        ],
    ),
];
