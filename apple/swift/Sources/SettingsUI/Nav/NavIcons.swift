// The icon of a section in the nav (`nav.rs` `section_icon`), as SF Symbols.

enum NavIcons {
    static func symbol(_ section: String) -> String {
        switch section {
        case "profile": "person.crop.circle"
        case "changed": "arrow.counterclockwise"
        case "appearance": "paintpalette"
        case "fonts": "textformat.size"
        case "motion": "play"
        case "terminal": "apple.terminal"
        case "sessions": "sidebar.left"
        case "git": "arrow.triangle.pull"
        case "agents": "sparkles"
        case "import": "square.and.arrow.down"
        case "projects": "folder"
        case "extensions": "puzzlepiece.extension"
        case "featureFlags": "testtube.2"
        case "notifications": "bell"
        case "menuBar": "menubar.rectangle"
        case "hotkey": "command"
        case "updates": "arrow.down.circle"
        case "advanced": "slider.horizontal.3"
        default: "gearshape"
        }
    }
}
