// The app's icons as SF Symbols: one name per meaning, so a screen says `Icon.reset` and never repeats a symbol string.
// Every raw value must resolve on macOS 26 (`IconTests` checks them against `NSImage(systemSymbolName:)`). Brand logos
// and anything without a symbol stay drawings elsewhere.

import SwiftUI

public enum Icon: String, CaseIterable, Sendable {
    // MARK: Settings sections (`nav.rs` `section_icon`)
    case profile = "person.crop.circle"
    case changed = "arrow.uturn.backward.circle"
    case appearance = "paintpalette"
    case fonts = "textformat.size"
    case motion = "play.circle"
    case terminal = "apple.terminal"
    case sessions = "sidebar.left"
    case git = "arrow.triangle.pull"
    case agents = "sparkles"
    case importSettings = "square.and.arrow.down"
    case projects = "folder.badge.gearshape"
    case extensions = "puzzlepiece.extension"
    case featureFlags = "testtube.2"
    case notifications = "bell"
    case menuBar = "menubar.rectangle"
    case hotkey = "command"
    case updates = "arrow.down.circle"
    case advanced = "slider.horizontal.3"
    case settings = "gearshape"
    // MARK: Actions and states
    case info = "info.circle"
    case infoFilled = "info.circle.fill"
    case warning = "exclamationmark.triangle.fill"
    case reset = "arrow.counterclockwise"
    case refresh = "arrow.clockwise"
    case add = "plus"
    case remove = "xmark"
    case clear = "xmark.circle.fill"
    case search = "magnifyingglass"
    case check = "checkmark"
    case copy = "doc.on.doc"
    case folder = "folder"
    case reveal = "arrow.up.forward.square"
    case goTo = "arrow.right"
    case disclosure = "chevron.right"
    case chooser = "chevron.up.chevron.down"
    case grip = "line.3.horizontal"
    case trash = "trash"
    case install = "arrow.down.to.line"
    case touchID = "touchid"
    case openTerminal = "terminal"

    /// The icon of a settings section by its schema id; the gear for one this build does not know.
    public init(section id: String) {
        self = [
            "profile": .profile, "changed": .changed, "appearance": .appearance, "fonts": .fonts, "motion": .motion,
            "terminal": .terminal, "sessions": .sessions, "git": .git, "agents": .agents, "import": .importSettings,
            "projects": .projects, "extensions": .extensions, "featureFlags": .featureFlags,
            "notifications": .notifications, "menuBar": .menuBar, "hotkey": .hotkey, "updates": .updates,
            "advanced": .advanced,
        ][id] ?? .settings
    }
}

extension Image {
    /// The symbol of `icon`.
    public init(_ icon: Icon) { self.init(systemName: icon.rawValue) }
}
