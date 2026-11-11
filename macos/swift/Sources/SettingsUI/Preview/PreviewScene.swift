import WordsModel
// Which scene the strip above a page shows (`preview_column.rs` `PreviewKind`) and what it says of itself.
// The strip holds nothing to operate: no focus stop, and one labelled group for VoiceOver.

enum PreviewScene: Equatable {
    case window, fonts, sidebar, agents, notifications, menuBar, hotkey, git

    init?(section: String) {
        switch section {
        case "appearance": self = .window
        case "fonts": self = .fonts
        case "sessions": self = .sidebar
        case "agents": self = .agents
        case "notifications": self = .notifications
        case "menuBar": self = .menuBar
        case "hotkey": self = .hotkey
        case "git": self = .git
        default: return nil
        }
    }

    var title: String {
        switch self {
        case .window: Words.text("settings-window-window", fallback: "Window")
        case .fonts: Words.text("settings-window-fonts", fallback: "Fonts")
        case .sidebar, .agents: Words.text("settings-window-terminals-list", fallback: "Terminals list")
        case .notifications: Words.text("settings-window-notifications", fallback: "Notifications")
        case .menuBar: Words.text("settings-window-menu-bar", fallback: "Menu bar")
        case .hotkey: Words.text("settings-window-hotkey", fallback: "Hotkey")
        case .git: Words.text("settings-section-git", fallback: "Changes & diffs")
        }
    }

    /// The line under the stage where the picture needs a word of how it differs from the app.
    var caption: String? {
        switch self {
        case .window: Words.text("settings-appearance-stage-caption", fallback: "Live preview of a window. Its blur isn't drawn here")
        case .fonts: Words.text("settings-fonts-hover-a-font-in-a-list", fallback: "Your fonts in a sample of terminal text")
        case .agents: Words.text("settings-agents-preview-caption", fallback: "The Terminals list with agents at work")
        case .notifications: Words.text("settings-notifications-preview-caption", fallback: "How a notification looks")
        default: nil
        }
    }

    var spoken: String {
        Words.text(
            "settings-window-preview-label", ["scene": .text(title)], fallback: "Preview of \(title) with your current settings")
    }
}
