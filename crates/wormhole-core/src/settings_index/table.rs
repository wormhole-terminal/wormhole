//! The table: what every section and setting is called and says.

use super::{Entry, Group, NavGroup, SectionId, SectionInfo};

const fn entry(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    keywords: &'static str,
    keys: &'static [&'static str],
) -> Entry {
    Entry { id, label, description, keywords, keys }
}

const fn group(title: &'static str, entries: &'static [Entry]) -> Group {
    Group { title, blurb: "", aside: "", entries }
}

pub(super) fn section(id: SectionId) -> &'static SectionInfo {
    match id {
        SectionId::Profile => &SectionInfo {
            title: "Profile",
            intro: "Every page here is this profile's. A new profile starts as a copy of one you have, \
                    without its terminals or agent logins.",
            nav: NavGroup::Profile,
        },
        SectionId::Appearance => &SectionInfo {
            title: "Appearance",
            intro: "What every window looks like. Changes apply as you make them.",
            nav: NavGroup::Look,
        },
        SectionId::Fonts => &SectionInfo {
            title: "Fonts",
            intro: "Terminal first: it's what you read most. Code follows it and Interface uses the \
                    system font until you pick one.",
            nav: NavGroup::Look,
        },
        SectionId::Terminal => &SectionInfo {
            title: "Terminal",
            intro: "How the terminal behaves. Its font is in Fonts.",
            nav: NavGroup::Work,
        },
        SectionId::Sessions => &SectionInfo {
            title: "Terminals & sidebar",
            intro: "The Terminals list on the left: how it's grouped and what each row shows.",
            nav: NavGroup::Work,
        },
        SectionId::Git => &SectionInfo {
            title: "Changes & diffs",
            intro: "The Changes panel and the diff view.",
            nav: NavGroup::Work,
        },
        SectionId::Agents => &SectionInfo {
            title: "Coding agents",
            intro: "Wormhole recognises the agent running in each terminal. With status hooks it \
                    knows exactly when it's working, waiting for you, or done.",
            nav: NavGroup::Agents,
        },
        SectionId::Projects => &SectionInfo {
            title: "Project folders",
            intro: "Change the defaults for every project, or pick one to set it up just for you.",
            nav: NavGroup::Preview,
        },
        SectionId::FeatureFlags => &SectionInfo {
            title: "Feature flags",
            intro: "Features still being built. They show here in a development build and on the \
                    Unstable update channel; otherwise a release has them off until they're finished.",
            nav: NavGroup::Preview,
        },
        SectionId::Notifications => &SectionInfo {
            title: "Notifications",
            intro: "What Wormhole tells you about terminals you aren't looking at.",
            nav: NavGroup::App,
        },
        SectionId::MenuBar => &SectionInfo {
            title: "Menu bar & hotkey",
            intro: "Wormhole in the macOS menu bar, and a shortcut that works from any app.",
            nav: NavGroup::App,
        },
        SectionId::Updates => &SectionInfo {
            title: "Updates",
            intro: "Signed releases, checked in the background.",
            nav: NavGroup::App,
        },
        SectionId::Advanced => &SectionInfo {
            title: "Advanced",
            intro: "Files, folders and a way back to the defaults.",
            nav: NavGroup::App,
        },
    }
}

pub(super) fn groups(id: SectionId) -> &'static [Group] {
    match id {
        SectionId::Profile => PROFILE,
        SectionId::Appearance => APPEARANCE,
        SectionId::Fonts => FONTS,
        SectionId::Terminal => TERMINAL,
        SectionId::Sessions => SESSIONS,
        SectionId::Git => GIT,
        SectionId::Agents => AGENTS,
        SectionId::Projects => PROJECTS,
        SectionId::Notifications => NOTIFICATIONS,
        SectionId::MenuBar => MENU_BAR,
        SectionId::FeatureFlags => FEATURE_FLAGS,
        SectionId::Updates => UPDATES,
        SectionId::Advanced => ADVANCED,
    }
}

const PROFILE: &[Group] = &[
    group(
        "",
        &[
            entry(
                "profile-name",
                "Name and colour",
                "",
                "profile rename avatar letter swatch colour color edge",
                &[],
            ),
            entry(
                "profile-start",
                "Start new terminals in",
                "",
                "folder directory cwd home path open also claim",
                &[],
            ),
            entry(
                "profile-env",
                "Environment",
                "Variables this profile's terminals start with",
                "env variable aws path registry export",
                &[],
            ),
            entry(
                "profile-git",
                "Git identity",
                "Set in its terminals only; Wormhole never writes to a repository.",
                "author committer name email commit user",
                &[],
            ),
            entry(
                "profile-logins",
                "Agent logins",
                "A separate login for each agent.",
                "claude codex account config dir home sign in",
                &[],
            ),
        ],
    ),
    group(
        "Danger zone",
        &[entry(
            "profile-delete",
            "Delete profile",
            "Its terminals, notes, tasks and settings are removed and its windows close.",
            "remove danger",
            &[],
        )],
    ),
];

const APPEARANCE: &[Group] = &[
    group("", &[entry("appearance-preview", "Live preview", "", "preview wallpaper window", &[])]),
    group(
        "Theme",
        &[
            entry(
                "theme-mode",
                "Mode",
                "Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode",
                "dark light system appearance match night day",
                &["theme_mode"],
            ),
            entry(
                "theme",
                "Theme",
                "",
                "colour color scheme palette dark light studio import ghostty iterm2 alacritty",
                &["theme", "theme_light"],
            ),
        ],
    ),
    group(
        "Accent",
        &[entry(
            "primary-color",
            "Use the theme's primary colour",
            "Off: a colour of your choice for buttons, focus rings, selection and links, in every theme",
            "accent highlight color colour swatch custom own",
            &["theme_primary", "primary_color"],
        )],
    ),
    group(
        "Window",
        &[
            entry(
                "opacity",
                "Background opacity",
                "30–100 %. Below 100 % the desktop shows through",
                "transparency translucent see through",
                &["opacity"],
            ),
            entry(
                "blur",
                "Blur what's behind the window",
                "Only visible below 100 % opacity",
                "vibrancy frosted",
                &["blur"],
            ),
            entry(
                "liquid-glass",
                "Liquid Glass",
                "Glass under the sidebar's and the panel's cards on macOS 26 and later. Off: every \
                 surface painted.",
                "glass translucent material apple tahoe cards",
                &["liquid_glass"],
            ),
            entry(
                "translucent",
                "Also translucent",
                "Use the opacity and blur above for these windows too",
                "settings theme studio window",
                &["settings_translucent", "studio_translucent"],
            ),
            entry(
                "title-border",
                "Line under the title row",
                "Between the bar and the terminal: always, only when output is scrolled under the bar, \
                 or never",
                "border hairline rule separator title bar toolbar",
                &["title_border"],
            ),
            entry(
                "column-dividers",
                "Column dividers",
                "The lines between the sidebar, the terminal and the right panel. Off hides them in \
                 every theme; the edges still resize. Increase Contrast keeps them",
                "border hairline rule separator edge columns divider",
                &["column_dividers"],
            ),
            entry(
                "status-bar",
                "Show status bar",
                "Branch, agents and terminal size along the bottom of every window",
                "bottom bar footer",
                &["status_bar"],
            ),
        ],
    ),
    group(
        "Title row",
        &[
            entry(
                "title-branch-status",
                "Branch status",
                "A dot while the tree has uncommitted changes, and how many commits the branch is ahead \
                 of or behind its upstream (↑2 ↓1)",
                "git dirty uncommitted ahead behind upstream push pull sync commits chip",
                &["title_branch_status"],
            ),
            entry(
                "title-agent-gauge",
                "Agent model and context",
                "The model a coding agent runs and how much of its context window is used, when the \
                 agent reports it (Claude Code does, through its status line)",
                "claude model context window tokens percent compact meter usage",
                &["title_agent_gauge"],
            ),
            entry(
                "title-ports",
                "Open ports",
                "The ports a terminal's programs listen on, as chips that open localhost in your browser",
                "localhost server dev http listen browser network",
                &["title_ports"],
            ),
            entry(
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
    group(
        "File icons",
        &[
            entry(
                "icon-theme",
                "File icons",
                "In the Changes and History panels. Zed icon themes work too",
                "icon theme",
                &["icon_theme"],
            ),
            entry(
                "icon-themes-folder",
                "Icon themes folder",
                "Put a Zed icon theme folder here; it's listed above within seconds",
                "zed folder",
                &[],
            ),
        ],
    ),
];

const FONTS: &[Group] = &[
    group("", &[entry("font-specimen", "Specimen", "", "preview sample", &[])]),
    group(
        "Terminal",
        &[
            entry(
                "font-terminal",
                "Font",
                "The terminal grid",
                "terminal font typeface size monospace jetbrains bundled",
                &["font_family", "font_size"],
            ),
            entry(
                "line-height",
                "Line height",
                "Terminal rows, as a multiple of the font size",
                "spacing leading",
                &["line_height"],
            ),
            entry(
                "font-ligatures",
                "Ligatures",
                "Join sequences like => and != into one glyph",
                "ligature",
                &["font_ligatures"],
            ),
        ],
    ),
    Group {
        title: "Code",
        blurb: "",
        aside: "Follows the terminal until you pick one",
        entries: &[entry(
            "font-code",
            "Font",
            "Diffs, commit hashes, file lists, Theme Studio",
            "code font typeface size monospace diff",
            &["code_font_family", "code_font_size"],
        )],
    },
    group(
        "Interface",
        &[entry(
            "font-interface",
            "Font",
            "Sidebar, panels, settings and controls. The size scales the whole interface",
            "interface font typeface size system ui",
            &["ui_font_family", "ui_font_size"],
        )],
    ),
    group(
        "Your fonts",
        &[
            entry(
                "fonts-folder",
                "Fonts folder",
                "JetBrains Mono comes with Wormhole. To use other fonts without installing them, put \
                 .ttf, .otf or .ttc files (or folders of them) here: they're listed under Custom within \
                 seconds",
                "custom bundled ttf otf ttc folder",
                &[],
            ),
            entry("custom-fonts", "Custom fonts", "", "custom families", &[]),
        ],
    ),
];

const TERMINAL: &[Group] = &[
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

const SESSIONS: &[Group] = &[
    group("", &[entry("sidebar-preview", "Live preview", "", "preview sidebar", &[])]),
    group(
        "Terminals in",
        &[entry(
            "terminals-in",
            "Show terminals in",
            "Sidebar: the list on the left; Tabs: a tab bar instead, with the groups as sets of tabs",
            "tabs tab bar sidebar list layout",
            &["terminals_in"],
        )],
    ),
    Group {
        title: "Terminals list",
        blurb: "",
        aside: "",
        entries: &[
            entry(
                "density",
                "Density",
                "How much room each terminal takes, in the sidebar and the tab bar: Regular rows are 48 pt with a second line, Compact 32, Dense 26",
                "density compact dense regular comfortable rows size",
                &["density"],
            ),
            entry(
                "cards",
                "Show groups as cards",
                "Off: groups are a head and their rows, with no card around them",
                "cards groups card background border",
                &["show_groups_as_cards"],
            ),
            entry(
                "sidebar-numbers",
                "⌘1–⌘9 numbers",
                "Stay with the terminal: dealt down the list, then kept (a new terminal gets the next one); Follow the list: the first nine rows you can see",
                "command shortcuts numbers stay follow list stable classic",
                &["sidebar_numbers"],
            ),
            entry(
                "cmd-hints",
                "Show ⌘1–⌘9 while ⌘ is held",
                "Keycaps on the first nine terminals after 250 ms; the shortcuts work either way",
                "command keycaps shortcuts numbers hints",
                &["sidebar_cmd_hints"],
            ),
        ],
    },
    group(
        "Row details",
        &[
            entry(
                "sidebar-changes",
                "Changed files",
                "±N uncommitted changes in the terminal's row",
                "git changes count",
                &["sidebar_changes"],
            ),
            entry(
                "sidebar-dot",
                "Status dot",
                "Working, needs you, finished or idle, as the state round every terminal's status tile",
                "status glyph agent",
                &["sidebar_status_dot"],
            ),
            entry(
                "sidebar-notes",
                "Notes and tasks",
                "Note icon with the number of notes and open tasks",
                "notes tasks next steps count",
                &["sidebar_notes"],
            ),
        ],
    ),
];

const GIT: &[Group] = &[
    group(
        "Changes panel",
        &[
            entry(
                "changes-view",
                "File view",
                "A flat list, or folders as a tree",
                "folders files list tree",
                &["changes_tree"],
            ),
            entry(
                "group-untracked",
                "Group untracked files separately",
                "New files get their own section under the changes",
                "untracked new files",
                &["group_untracked"],
            ),
        ],
    ),
    group(
        "Diff view",
        &[
            entry(
                "diff-layout",
                "Default layout",
                "Split keeps old and new side by side, even for a new or deleted file. The diff's own \
                 toolbar switches one diff",
                "split unified side by side",
                &["diff_split"],
            ),
            entry(
                "diff-wrap",
                "Wrap long lines",
                "Otherwise the diff scrolls sideways",
                "wrap",
                &["diff_wrap"],
            ),
            entry("tab-width", "Tab width", "Spaces per tab in diffs", "indent tabs", &["tab_width"]),
            entry(
                "size-units",
                "File sizes",
                "Decimal like Finder (12.4 MB), or binary like git (11.8 MiB)",
                "size units bytes kilobytes megabytes kb mb kib mib finder binary decimal",
                &["size_units"],
            ),
        ],
    ),
];

const AGENTS: &[Group] = &[
    group("", &[entry("agents-summary", "Agents found", "", "installed hooks repair path summary", &[])]),
    group(
        "Notifications",
        &[entry(
            "agents-notifications",
            "Notify me when an agent needs you or finishes",
            "These, and notifications about long commands and the bell, have their own section",
            "notifications alerts banner",
            &[],
        )],
    ),
    group(
        "Installed",
        &[entry(
            "agents-installed",
            "Installed agents",
            "",
            "claude code codex gemini copilot cursor opencode aider amp goose qwen hooks status install \
             remove repair",
            &[],
        )],
    ),
    group(
        "Other agents",
        &[entry(
            "agents-other",
            "Other agents",
            "Agents Wormhole recognises that aren't on this Mac's PATH",
            "not installed path",
            &[],
        )],
    ),
    group(
        "How status works",
        &[entry(
            "agents-how",
            "How status works",
            "With status hooks the agent tells Wormhole the moment it starts working, waits for you or \
             finishes; without them the status is read from the terminal output",
            "hooks output heuristic backup",
            &[],
        )],
    ),
    Group {
        title: "Command line",
        blurb: "Scripts and agents can drive Wormhole from the command line: list terminals, read a \
                status and send notifications.",
        aside: "",
        entries: &[
            // The command's name is on the row itself (`ipc::control::COMMAND_NAME`); the keywords
            // carry the names it has been called, so a search finds the row by either.
            entry(
                "cli-tool",
                "Command line tool",
                "The Wormhole command in any terminal",
                "wh wht cli command install uninstall path shell alias symlink bin usr local terminal",
                &[],
            ),
            entry(
                "control-write",
                "Let the command line type and open terminals",
                "send-text types into a terminal and new-terminal opens one, as a paste; never into the \
                 terminal that asks, nor one waiting for your answer",
                "cli command line control api socket send-text new-terminal scripts agents automation \
                 remote type",
                &["control_write"],
            ),
        ],
    },
];

const PROJECTS: &[Group] = &[group(
    "",
    &[entry(
        "projects",
        "Project folders",
        "The .wormhole folder of a project: trust, and which of its files you use",
        ".wormhole project folder trust touch id actions prompts layout notes theme settings",
        &[],
    )],
)];

const FEATURE_FLAGS: &[Group] = &[group(
    "",
    &[entry(
        "feature-flags",
        "Feature flags",
        "Turn features that are still being built on or off in this build",
        "feature flag preview experimental development dev in progress beta unstable project folders",
        &["feature_flags"],
    )],
)];

const NOTIFICATIONS: &[Group] = &[
    group(
        "macOS",
        &[entry(
            "notify-permission",
            "Notifications in System Settings",
            "",
            "permission allow system settings send test banner",
            &[],
        )],
    ),
    group(
        "Notify me when",
        &[
            entry(
                "notify-needs-you",
                "A terminal needs you",
                "An agent asks a question or for permission",
                "time sensitive focus question permission notifications",
                &["notify_needs_you"],
            ),
            entry(
                "notify-agent-done",
                "An agent finishes",
                "Its Stop hook, or its output settles. A program that reports it is done or failed counts too",
                "notifications done program report failed error",
                &["notify_agent_done"],
            ),
            entry(
                "notify-command-done",
                "A command finishes",
                "Any foreground command, agent or not, that ran longer than",
                "long build threshold seconds notifications",
                &["notify_command_done", "notify_command_after"],
            ),
            entry(
                "notify-command-failed",
                "A command fails",
                "Non-zero exit code. Needs shell integration (on for zsh, bash and fish)",
                "failed failure error exit code build notifications",
                &["notify_command_failed"],
            ),
            entry(
                "notify-bell",
                "The terminal rings the bell",
                "Terminals without an agent; at most once a minute each",
                "bell notifications",
                &["notify_bell"],
            ),
        ],
    ),
    group(
        "Delivery",
        &[
            entry(
                "notify-only-unseen",
                "Only for terminals I’m not looking at",
                "Off: the terminal in front notifies too",
                "unseen front",
                &["notify_only_unseen"],
            ),
            entry(
                "notify-in-front",
                "While Wormhole is in front",
                "A system banner, or nothing",
                "banner nothing front",
                &["notify_in_front"],
            ),
            entry("notify-sound", "Sound", "", "sound alert", &["notify_sound"]),
            entry(
                "notify-group-bursts",
                "Group bursts",
                "Three or more within 10 s become one summary",
                "summary bursts",
                &["notify_group_bursts"],
            ),
        ],
    ),
    group(
        "Dock",
        &[
            entry("dock-badge", "Badge", "The count on the Dock icon", "dock badge count", &["dock_badge"]),
            entry(
                "dock-bounce",
                "Bounce once when a terminal needs you",
                "Only while Wormhole isn’t the active app; never for a bell",
                "dock bounce",
                &["dock_bounce"],
            ),
        ],
    ),
];

const MENU_BAR: &[Group] = &[
    group(
        "Menu bar",
        &[
            entry(
                "menu-bar",
                "Show Wormhole in the menu bar",
                "Terminals by status and quick actions",
                "status bar tray item menubar",
                &["menu_bar"],
            ),
            entry(
                "menu-bar-icon",
                "Icon shows",
                "The dot and count on the status item",
                "status item icon count",
                &["menu_bar_icon"],
            ),
            entry(
                "menu-bar-animate",
                "Animate menu bar icon while working",
                "The prompt in the icon types itself out while something runs. Off: a still, filled icon",
                "animation",
                &["menu_bar_animate"],
            ),
        ],
    ),
    group(
        "Global hotkey",
        &[
            entry(
                "hotkey",
                "Shortcut",
                "Works from any app. Click, then press the keys",
                "global shortcut quake record hotkey",
                &["hotkey"],
            ),
            entry(
                "hotkey-action",
                "Hotkey opens",
                "Pressing it again with the panel open shows Wormhole",
                "quick panel toggle",
                &["hotkey_action"],
            ),
            entry(
                "jump-waiting-key",
                "Jump to next waiting terminal",
                "Inside Wormhole, in any window",
                "needs you waiting shortcut",
                &["jump_waiting_key"],
            ),
        ],
    ),
];

const UPDATES: &[Group] = &[group(
    "Updates",
    &[
        entry("version", "Version", "", "release build", &[]),
        entry(
            "auto-update",
            "Update automatically",
            "Checks in the background and installs on the next restart",
            "download auto",
            &["auto_update"],
        ),
        entry(
            "update-channel",
            "Channel",
            "Unstable also gets a build of every change once it passes the tests, and shows the \
             feature flags",
            "beta nightly unstable stable prerelease dev builds main feature flags",
            &["update_channel"],
        ),
        entry("update-status", "Check for updates", "", "install restart release notes check skip", &[]),
    ],
)];

const ADVANCED: &[Group] = &[
    group(
        "Files",
        &[
            entry("open-config", "Settings file", "config.json: edits apply live", "json config", &[]),
            entry("open-data", "Data folder", "Terminals, notes, themes, logs", "finder database", &[]),
            entry("open-logs", "Logs", "Set WORMHOLE_LOG=debug for more detail", "debug log", &[]),
            entry(
                "reset-settings",
                "Reset all settings",
                "Your terminals, notes and themes are kept",
                "defaults reset restore",
                &[],
            ),
        ],
    ),
    group(
        "Tips",
        &[entry(
            "reset-tips",
            "Closed tips",
            "Bring back the tips closed with “Don't show again”, such as the sidebar's coding agents note",
            "hints tips reset show again dismissed",
            &[],
        )],
    ),
];
