// The rows the window shows, by page. A host replaces a row's options and values through the store; this is the layout.
let standardPages: [Page] = [
  Page(id: "Profile", title: "Profile", preview: "", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "", rows: []),
    SettingGroup(title: "", rows: []),
  ]),
  Page(id: "Terminal", title: "Terminal", preview: "terminal", notice: nil, footer: "The worktree folder and the shells list are in config.json.", footerButtons: ["Edit config.json"], groups: [
    SettingGroup(title: "Behaviour", rows: [
      Row("Scrollback", "Applies to new terminals", .popup, options: ["10,000 lines", "50,000 lines", "100,000 lines", "Unlimited"], value: "", info: ""),
      Row("Restore text after restart", "Encrypted, on this Mac only. Off erases what’s saved", .toggle, options: [], value: "on", info: ""),
      Row("Minimap", "", .toggle, options: [], value: "off", info: "The scrollback drawn small along the terminal's right edge. Click or drag it to scroll."),
      Row("Gaps between split terminals", "", .toggle, options: [], value: "on", info: ""),
      Row("Option key as Meta", "A side set to Meta can’t type @ { [ on some layouts", .popup, options: ["Off", "Left", "Right", "Both"], value: "", info: ""),
      Row("Font", "", .link, options: [], value: "", info: "", fontLink: true),
    ]),
    SettingGroup(title: "Opening", rows: [
      Row("Open folders in", "", .popup, options: ["Zed", "Visual Studio Code", "Cursor", "Xcode", "Finder", "-", "Choose app…"], value: "", info: "", appIcons: true),
      Row("Default terminal", "Opens .command files and scripts", .button, options: [], value: "Make Default", info: ""),
      Row("Shell", "", .popup, options: ["Login shell (zsh)", "bash", "fish", "-", "Edit shells…"], value: "", info: ""),
      Row("Copy build folders into new worktrees", "", .toggle, options: [], value: "off", info: "Clones the main checkout's node_modules and target into a new worktree, so builds start warm. Only on the same APFS volume."),
    ]),
    SettingGroup(title: "Bell", rows: [
      Row("Sound", "", .toggle, options: [], value: "on", info: ""),
      Row("Flash the terminal edge", "", .toggle, options: [], value: "off", info: ""),
    ]),
    SettingGroup(title: "Privacy", rows: [
      Row("Programs reading the clipboard", "", .popup, options: ["Ask", "Allow", "Deny"], value: "", info: "OSC 52, over ssh too. The clipboard often holds a password."),
      Row("Programs setting the clipboard", "", .popup, options: ["Allow", "Deny"], value: "", info: ""),
      Row("Screen read-back", "", .toggle, options: [], value: "off", info: "DECRQCRA: lets a program read the screen cell by cell, over ssh too."),
      Row("Window control", "", .toggle, options: [], value: "off", info: "xterm's window operations: move, resize, minimise or raise the window."),
      Row("Title read-back", "", .toggle, options: [], value: "off", info: "XTWINOPS 20 and 21. Over ssh this lets a program type a command line."),
    ]),
  ]),
  Page(id: "Appearance", title: "Appearance", preview: "window", notice: nil, footer: "Add an icon theme by putting it in the icon themes folder.", footerButtons: ["Show in Finder"], groups: [
    SettingGroup(title: "Theme", rows: [
      Row("Mode", "", .popup, options: ["settings_appearance_match_system|Match system", "settings_appearance_dark|Dark", "settings_appearance_light|Light"], value: "", info: ""),
      Row("Choose", "", .popup, options: ["pairing_both|One theme for both", "pairing_separate|Dark and light separately"], value: "", info: ""),
      Row("", "", .themes, options: [], value: "", info: ""),
    ]),
    SettingGroup(title: "Accent", rows: [
      Row("Primary colour", "", .swatches, options: [], value: "Theme", info: "Buttons, focus rings, selection and links, in every theme.", swatches: "primary"),
      Row("Branch colour", "", .swatches, options: [], value: "Theme", info: "The git branch on folders, in the title row and in the status bar. Any colour is kept readable.", swatches: "branch"),
    ]),
    SettingGroup(title: "Window", rows: [
      Row("Opacity", "", .popup, options: ["n100|100 %", "n95|95 %", "n90|90 %", "n80|80 %", "n70|70 %"], value: "", info: ""),
      Row("Blur behind the window", "Below 100 % opacity", .toggle, options: [], value: "on", info: "", warnWhenOff: "Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read."),
      Row("Liquid Glass", "", .toggle, options: [], value: "on", info: "Glass under the sidebar's and the panel's cards. Off: every surface painted.", chip: "Preview"),
      Row("Also translucent", "", .checks, options: [], value: "", info: "Use the opacity and blur above for these windows too.", checks: ["Settings", "Theme Studio"]),
    ]),
    SettingGroup(title: "Details", rows: [
      Row("Borders", "", .toggle, options: [], value: "off", info: "Lines around cards, columns and the sidebar. Off: surfaces are told apart by their shade alone."),
      Row("Column dividers", "", .toggle, options: [], value: "on", info: ""),
      Row("Status bar", "", .toggle, options: [], value: "on", info: ""),
      Row("Animate icons", "", .toggle, options: [], value: "on", info: "Buttons show what a click did with their icon. Reduce Motion turns this off too."),
    ]),
    SettingGroup(title: "Title row", rows: [
      Row("", "", .parts, options: [], value: "", info: ""),
    ]),
    SettingGroup(title: "File icons", rows: [
      Row("Icon theme", "Zed icon themes work too", .popup, options: ["Wormhole", "Material Icon Theme", "Catppuccin Icons"], value: "", info: ""),
    ]),
  ]),
  Page(id: "Fonts", title: "Fonts", preview: "specimen", notice: nil, footer: "None yet: the fonts folder is empty.", footerButtons: ["Show in Finder"], groups: [
    SettingGroup(title: "Terminal", rows: [
      Row("Font", "", .font, options: [], value: "JetBrains Mono", info: "", size: 14, mono: true),
      Row("Line height", "", .popup, options: ["1.30", "1.20", "1.40", "1.50"], value: "", info: ""),
      Row("Ligatures", "Shows => and != as one glyph", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Code", rows: [
      Row("Font", "Diffs, file lists, Theme Studio", .font, options: [], value: "same_as_terminal", info: "", size: 13, mono: true, same: true),
    ]),
    SettingGroup(title: "Interface", rows: [
      Row("Font", "Its size scales the interface", .font, options: [], value: "SF Pro", info: "", size: 13),
    ]),
  ]),
  Page(id: "Sessions", title: "Terminals & sidebar", preview: "sidebar", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Layout", rows: [
      Row("Show terminals in", "", .popup, options: ["Sidebar", "Tabs"], value: "", info: ""),
      Row("Density", "", .popup, options: ["regular|Regular", "compact|Compact", "dense|Dense"], value: "", info: ""),
    ]),
    SettingGroup(title: "Groups", rows: [
      Row("Groups as cards", "", .toggle, options: [], value: "on", info: ""),
      Row("Fade group colours", "", .toggle, options: [], value: "on", info: ""),
      Row("Show group folders", "", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Shortcuts", rows: [
      Row("⌘1–⌘9 numbers", "", .popup, options: ["Stay with the terminal", "Follow the list"], value: "", info: ""),
      Row("Show numbers while ⌘ is held", "", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Row details", rows: [
      Row("Changed files", "", .toggle, options: [], value: "on", info: ""),
      Row("Count", "", .popup, options: ["This terminal", "Whole repository"], value: "", info: "", needs: 0),
      Row("Plain terminals count", "", .popup, options: ["Commands that finish", "Any program"], value: "", info: "Which programs make a terminal without an agent count the files changed while they run.", needs: 0),
      Row("Files changed at once", "", .popup, options: ["Each counts it", "Nobody counts it"], value: "", info: "When several terminals worked at the same time, whose file it is can't be told.", needs: 0),
      Row("Remember after restart", "", .toggle, options: [], value: "on", info: "", needs: 0),
      Row("Status dot", "", .toggle, options: [], value: "on", info: ""),
      Row("Notes and tasks", "", .toggle, options: [], value: "on", info: ""),
    ]),
  ]),
  Page(id: "Git", title: "Changes & diffs", preview: "diff", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Changes panel", rows: [
      Row("File view", "", .popup, options: ["settings_git_list|List", "settings_git_tree|Tree"], value: "", info: ""),
      Row("Untracked files separately", "", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Diff view", rows: [
      Row("Layout", "", .popup, options: ["settings_git_split|Split", "settings_git_unified|Unified"], value: "", info: ""),
      Row("Wrap long lines", "", .toggle, options: [], value: "off", info: ""),
      Row("Tab width", "", .popup, options: ["n4|4", "n2|2", "n8|8"], value: "", info: ""),
      Row("File sizes", "", .popup, options: ["MB", "MiB"], value: "", info: "Decimal like Finder (12.4 MB), or binary like git (11.8 MiB)."),
    ]),
  ]),
  Page(id: "Notifications", title: "Notifications", preview: "banner", notice: ("warn", "Off in System Settings: Wormhole can’t show banners until you allow them.", "Open System Settings"), footer: "Banner style and Focus are set by macOS.", footerButtons: ["Send a test"], groups: [
    SettingGroup(title: "Notify me when", rows: [
      Row("A terminal needs input", "", .toggle, options: [], value: "on", info: ""),
      Row("An agent finishes", "", .toggle, options: [], value: "on", info: ""),
      Row("A long command finishes", "", .popup, options: ["n10|After 10 s", "n30|After 30 s", "n60|After 1 min", "settings_notifications_off|Off"], value: "", info: ""),
      Row("A command fails", "zsh, bash and fish", .toggle, options: [], value: "on", info: ""),
      Row("The bell rings", "", .toggle, options: [], value: "off", info: ""),
    ]),
    SettingGroup(title: "Delivery", rows: [
      Row("Only for terminals I’m not looking at", "", .toggle, options: [], value: "on", info: ""),
      Row("While Wormhole is in front", "", .popup, options: ["Banner", "Nothing"], value: "", info: ""),
      Row("Sound", "", .popup, options: ["Needs input only", "Always", "Never"], value: "", info: ""),
      Row("Group bursts", "Three or more in 10 s become one", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Dock", rows: [
      Row("Badge", "", .popup, options: ["Off", "Needs input", "Needs input and finished"], value: "", info: ""),
      Row("Bounce when a terminal needs input", "", .toggle, options: [], value: "off", info: ""),
    ]),
  ]),
  Page(id: "Agents", title: "Coding agents", preview: "agents", notice: nil, footer: "When an agent notifies you is set in Notifications.", footerButtons: ["Open Notifications", "How status works"], groups: [
    SettingGroup(title: "Agents on this Mac", rows: [
    ]),
    SettingGroup(title: "Command line", rows: [
      Row("Command line tool", "The wh command in any terminal", .button, options: [], value: "Install", info: ""),
      Row("Let it type and open terminals", "", .toggle, options: [], value: "off", info: "send-text types into a terminal and new-terminal opens one, as a paste; never into one waiting for your answer."),
    ]),
    SettingGroup(title: "Agent access", rows: [
      Row("Scripts and agents", "", .popup, options: ["Off", "Ask", "Allowed while open"], value: "", info: "What programs that drive Wormhole through its command line, MCP or socket may do."),
    ]),
    SettingGroup(title: "Skill and MCP", rows: [
    ]),
  ]),
  Page(id: "MenuBar", title: "Menu bar & hotkey", preview: "menubar", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Menu bar", rows: [
      Row("Show in the menu bar", "", .toggle, options: [], value: "on", info: ""),
      Row("Icon shows", "", .popup, options: ["settings_menu_bar_status_and_count|Status and count", "settings_menu_bar_status|Status", "settings_menu_bar_icon_only|Icon only"], value: "", info: ""),
      Row("Animate while working", "", .toggle, options: [], value: "on", info: ""),
    ]),
    SettingGroup(title: "Global hotkey", rows: [
      Row("Shortcut", "Works from any app", .key, options: [], value: "⌥Space", info: ""),
      Row("Opens", "", .popup, options: ["Quick panel", "Wormhole"], value: "", info: ""),
      Row("Next waiting terminal", "Inside Wormhole", .key, options: [], value: "", info: ""),
    ]),
  ]),
  Page(id: "Import", title: "Import", preview: "", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Found on this Mac", rows: [
    ]),
    SettingGroup(title: "", rows: [
      Row("Compare with Wormhole", "", .link, options: [], value: "", info: "", sheet: "import-compare"),
    ]),
  ]),
  Page(id: "Updates", title: "Updates", preview: "", notice: ("info", "Development build: updates only work in released builds.", ""), footer: "", footerButtons: [], groups: [
    SettingGroup(title: "", rows: [
      Row("", "", .update, options: [], value: "", info: ""),
    ]),
    SettingGroup(title: "", rows: [
      Row("Version", "", .value, options: [], value: "1.5.0", info: ""),
      Row("Update automatically", "Installs on the next restart", .toggle, options: [], value: "on", info: ""),
      Row("Channel", "", .popup, options: ["Stable", "Unstable"], value: "", info: "Unstable also gets a build of every change once it passes the tests, and shows the feature flags."),
    ]),
    SettingGroup(title: "Privacy", rows: [
      Row("Count this Mac anonymously", "", .toggle, options: [], value: "on", info: "No analytics, telemetry or crash reports. The first check of a day and of a month says so, with nothing that tells one Mac from another."),
      Row("What an update check sends", "", .link, options: [], value: "", info: "", sheet: "update-sends"),
    ]),
  ]),
  Page(id: "Advanced", title: "Advanced", preview: "", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Files", rows: [
      Row("Settings file", "config.json, edits apply live", .button, options: [], value: "Open", info: ""),
      Row("Data folder", "", .button, options: [], value: "Show in Finder", info: ""),
      Row("Logs", "", .button, options: [], value: "Show in Finder", info: ""),
    ]),
    SettingGroup(title: "", rows: [
      Row("Closed tips", "", .button, options: [], value: "Show Again", info: ""),
      Row("Skip malware scans of new programs", "", .button, options: [], value: "Open Developer Tools", info: "Adds Wormhole to Developer Tools, so programs started in it skip the first-run scan. Only if you trust what you run."),
    ]),
    SettingGroup(title: "", rows: [
      Row("Reset all settings", "Terminals, notes and themes are kept", .danger, options: [], value: "Reset…", info: "", confirm: "Reset all settings?", confirmText: "Every setting goes back to its default. Your terminals, notes and themes are kept.", resetAll: true, action: "Reset"),
    ]),
  ]),
  Page(id: "Motion", title: "Motion", preview: "motion", notice: nil, footer: "", footerButtons: [], groups: [
    SettingGroup(title: "Terminals list", rows: [
      Row("A terminal opens or closes", "", .popup, options: ["Classic", "Reveal", "From the side", "Soft spring"], value: "", info: ""),
      Row("A group folds", "", .popup, options: ["Classic", "Drawer", "Rows tuck in"], value: "", info: ""),
      Row("A status changes", "", .popup, options: ["Classic", "Crossfade", "Badge pops", "Card wash", "Amber flare", "Nudge", "Line rolls"], value: "", info: ""),
    ]),
    SettingGroup(title: "Window", rows: [
      Row("The sidebar and panel hide or show", "", .popup, options: ["Classic", "Soft spring", "Content after"], value: "", info: ""),
    ]),
  ], badge: true),
  Page(id: "Projects", title: "Project folders", preview: "", notice: nil, footer: "A project’s own choice is only for you: nothing is written into the project folder.", footerButtons: [], groups: [
    SettingGroup(title: "Projects", rows: [
    ]),
    SettingGroup(title: "Use from projects", rows: [
    ]),
  ], badge: true),
  Page(id: "Extensions", title: "Extensions", preview: "", notice: nil, footer: "Probes live in the Probes folder, one folder each with a probe.toml.", footerButtons: ["Show in Finder", "Scan Again"], groups: [
    SettingGroup(title: "Probes", rows: [
    ]),
  ], badge: true),
  Page(id: "FeatureFlags", title: "Feature flags", preview: "", notice: ("info", "Switches for features that aren’t finished. Only development builds show this page; a release uses each feature’s default.", ""), footer: "", footerButtons: [], groups: [
    SettingGroup(title: "", rows: [
      Row("Project folders", "", .toggle, options: [], value: "off", info: ""),
      Row("Terminal welcome", "", .toggle, options: [], value: "off", info: ""),
      Row("Notes and tasks", "", .toggle, options: [], value: "on", info: ""),
      Row("Persistent terminals", "", .toggle, options: [], value: "off", info: ""),
      Row("Automation", "", .toggle, options: [], value: "on", info: ""),
      Row("Motion", "", .toggle, options: [], value: "off", info: ""),
      Row("Tear off a terminal", "", .toggle, options: [], value: "on", info: ""),
      Row("Live resize", "", .toggle, options: [], value: "on", info: ""),
      Row("Notice chip", "", .toggle, options: [], value: "on", info: ""),
      Row("Probes", "", .toggle, options: [], value: "off", info: ""),
      Row("Files", "", .toggle, options: [], value: "on", info: ""),
      Row("Folder picker", "", .toggle, options: [], value: "off", info: ""),
      Row("Native Settings window", "", .toggle, options: [], value: "on", info: ""),
    ]),
  ], badge: true),
]

/// The title row's parts in board order; "-" is the divider. A part's note is shown under its name.
let titlePartLayout: [(name: String, on: Bool, note: String)] = [
  ("Folder", true, ""),
  ("Branch", true, ""),
  ("Branch status", true, ""),
  ("Terminal name", true, ""),
  ("Terminal status", true, ""),
  ("-", true, ""),
  ("Agent model and context", true, ""),
  ("Open ports", true, ""),
  ("Pull request", false, "Asks gh, only in a trusted project"),
  ("Open in…", true, ""),
  ("Split buttons", true, ""),
]

let previewCaptionOverride = ["specimen": "The font you chose, at its size and line height."]
