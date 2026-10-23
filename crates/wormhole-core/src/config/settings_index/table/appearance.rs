//! The Appearance and Fonts sections: theme, accent, window, title row, file icons and the type.

use super::{literal_entry, literal_group};
use crate::config::settings_index::{Group, Word};

pub(super) const APPEARANCE: &[Group] = &[
    literal_group(
        "",
        &[literal_entry("appearance-preview", "Live preview", "", "preview wallpaper window", &[])],
    ),
    literal_group(
        "Theme",
        &[
            literal_entry(
                "theme-mode",
                "Mode",
                "Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode",
                "dark light system appearance match night day",
                &["theme_mode"],
            ),
            literal_entry(
                "theme",
                "Theme",
                "",
                "colour color scheme palette dark light studio import ghostty iterm2 alacritty",
                &["theme", "theme_light"],
            ),
        ],
    ),
    literal_group(
        "Accent",
        &[literal_entry(
            "primary-color",
            "Use the theme's primary colour",
            "Off: a colour of your choice for buttons, focus rings, selection and links, in every theme",
            "accent highlight color colour swatch custom own",
            &["theme_primary", "primary_color"],
        )],
    ),
    literal_group(
        "Window",
        &[
            literal_entry(
                "opacity",
                "Background opacity",
                "30–100 %. Below 100 % the desktop shows through",
                "transparency translucent see through",
                &["opacity"],
            ),
            literal_entry(
                "blur",
                "Blur what's behind the window",
                "Only visible below 100 % opacity",
                "vibrancy frosted",
                &["blur"],
            ),
            literal_entry(
                "liquid-glass",
                "Liquid Glass",
                "Glass under the sidebar's and the panel's cards on macOS 26 and later. Off: every \
                 surface painted.",
                "glass translucent material apple tahoe cards",
                &["liquid_glass"],
            ),
            literal_entry(
                "translucent",
                "Also translucent",
                "Use the opacity and blur above for these windows too",
                "settings theme studio window",
                &["settings_translucent", "studio_translucent"],
            ),
            literal_entry(
                "title-border",
                "Line under the title row",
                "Between the bar and the terminal: always, only when output is scrolled under the bar, \
                 or never",
                "border hairline rule separator title bar toolbar",
                &["title_border"],
            ),
            literal_entry(
                "column-dividers",
                "Column dividers",
                "The lines between the sidebar, the terminal and the right panel. Off hides them in \
                 every theme; the edges still resize. Increase Contrast keeps them",
                "border hairline rule separator edge columns divider",
                &["column_dividers"],
            ),
            literal_entry(
                "status-bar",
                "Show status bar",
                "Branch, agents and terminal size along the bottom of every window",
                "bottom bar footer",
                &["status_bar"],
            ),
        ],
    ),
    literal_group(
        "Title row",
        &[
            literal_entry(
                "title-branch-status",
                "Branch status",
                "A dot while the tree has uncommitted changes, and how many commits the branch is ahead \
                 of or behind its upstream (↑2 ↓1)",
                "git dirty uncommitted ahead behind upstream push pull sync commits chip",
                &["title_branch_status"],
            ),
            literal_entry(
                "title-agent-gauge",
                "Agent model and context",
                "The model a coding agent runs and how much of its context window is used, when the \
                 agent reports it (Claude Code does, through its status line)",
                "claude model context window tokens percent compact meter usage",
                &["title_agent_gauge"],
            ),
            literal_entry(
                "title-ports",
                "Open ports",
                "The ports a terminal's programs listen on, as chips that open localhost in your browser",
                "localhost server dev http listen browser network",
                &["title_ports"],
            ),
            literal_entry(
                "title-pull-request",
                "Pull request",
                "The branch's pull request and how its checks and reviews stand: a link in the title \
                 row and a mark on the folder's band in the sidebar, with a notification when its \
                 checks finish. Asked of the GitHub CLI (gh), only in a trusted project, and only when \
                 gh is installed and signed in",
                "github gh pr merge review link ci checks actions build failed passed draft approved",
                &["title_pull_request"],
            ),
        ],
    ),
    literal_group(
        "File icons",
        &[
            literal_entry(
                "icon-theme",
                "File icons",
                "In the Changes and History panels. Zed icon themes work too",
                "icon theme",
                &["icon_theme"],
            ),
            literal_entry(
                "icon-themes-folder",
                "Icon themes folder",
                "Put a Zed icon theme folder here; it's listed above within seconds",
                "zed folder",
                &[],
            ),
        ],
    ),
];

pub(super) const FONTS: &[Group] = &[
    literal_group("", &[literal_entry("font-specimen", "Specimen", "", "preview sample", &[])]),
    literal_group(
        "Terminal",
        &[
            literal_entry(
                "font-terminal",
                "Font",
                "The terminal grid",
                "terminal font typeface size monospace jetbrains bundled",
                &["font_family", "font_size"],
            ),
            literal_entry(
                "line-height",
                "Line height",
                "Terminal rows, as a multiple of the font size",
                "spacing leading",
                &["line_height"],
            ),
            literal_entry(
                "font-ligatures",
                "Ligatures",
                "Join sequences like => and != into one glyph",
                "ligature",
                &["font_ligatures"],
            ),
        ],
    ),
    Group {
        title: Word::Literal("Code"),
        blurb: Word::Literal(""),
        aside: Word::Literal("Follows the terminal until you pick one"),
        entries: &[literal_entry(
            "font-code",
            "Font",
            "Diffs, commit hashes, file lists, Theme Studio",
            "code font typeface size monospace diff",
            &["code_font_family", "code_font_size"],
        )],
    },
    literal_group(
        "Interface",
        &[literal_entry(
            "font-interface",
            "Font",
            "Sidebar, panels, settings and controls. The size scales the whole interface",
            "interface font typeface size system ui",
            &["ui_font_family", "ui_font_size"],
        )],
    ),
    literal_group(
        "Your fonts",
        &[
            literal_entry(
                "fonts-folder",
                "Fonts folder",
                "JetBrains Mono comes with Wormhole. To use other fonts without installing them, put \
                 .ttf, .otf or .ttc files (or folders of them) here: they're listed under Custom within \
                 seconds",
                "custom bundled ttf otf ttc folder",
                &[],
            ),
            literal_entry("custom-fonts", "Custom fonts", "", "custom families", &[]),
        ],
    ),
];
