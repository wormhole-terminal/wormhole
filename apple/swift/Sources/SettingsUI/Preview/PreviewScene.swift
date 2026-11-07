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
        case .window: String(localized: "settings-window-window", defaultValue: "Window")
        case .fonts: String(localized: "settings-window-fonts", defaultValue: "Fonts")
        case .sidebar, .agents: String(localized: "settings-window-terminals-list", defaultValue: "Terminals list")
        case .notifications: String(localized: "settings-window-notifications", defaultValue: "Notifications")
        case .menuBar: String(localized: "settings-window-menu-bar", defaultValue: "Menu bar")
        case .hotkey: String(localized: "settings-window-hotkey", defaultValue: "Hotkey")
        case .git: String(localized: "settings-section-git", defaultValue: "Changes & diffs")
        }
    }

    /// The line under the stage where the picture needs a word of how it differs from the app.
    var caption: String? {
        switch self {
        case .window: String(localized: "settings-appearance-stage-caption", defaultValue: "Live preview of a window. Its blur isn't drawn here")
        case .fonts: String(localized: "settings-fonts-hover-a-font-in-a-list", defaultValue: "Your fonts in a sample of terminal text")
        case .agents: String(localized: "settings-agents-preview-caption", defaultValue: "The Terminals list with agents at work")
        case .notifications: String(localized: "settings-notifications-preview-caption", defaultValue: "How a notification looks")
        default: nil
        }
    }

    var spoken: String {
        String(localized: "settings-window-preview-label", defaultValue: "Preview of \(title) with your current settings")
    }
}
