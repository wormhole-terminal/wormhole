// The glyph in front of a terminal row of the status item's menu: the shapes of the sidebar's lead column,
// drawn as SF Symbols in the theme's status colours (`menubar::model::Glyph`).

import Foundation

public enum StatusGlyph: Sendable, Hashable {
    /// An agent or a program that reports needs input: an amber dot.
    case needsYou
    /// A terminal nothing tells the state of rang its bell: an amber bell.
    case bell
    case working
    /// A command runs in a terminal nothing tells the state of: the same, neutral.
    case running
    case done
    case failed
    case idle
    case exited

    public init(_ terminal: AmbientTerminal) {
        switch (terminal.state, terminal.source.told) {
        case (.needsYou, true): self = .needsYou
        case (.needsYou, false): self = .bell
        case (.working, true): self = .working
        case (.working, false): self = .running
        case (.done, _): self = .done
        case (.failed, _): self = .failed
        case (.idle, _): self = .idle
        case (.exited, _): self = .exited
        }
    }

    /// The SF Symbol, and the theme token that colours it (nil: the menu's own text colour).
    public var symbol: (name: String, token: String?) {
        switch self {
        case .needsYou: ("circle.fill", "agent.attention")
        case .bell: ("bell.fill", "agent.attention")
        case .working: ("circle.dotted", "agent.working")
        case .running: ("play.fill", "text.muted")
        case .done: ("checkmark", "agent.done")
        case .failed: ("xmark.circle.fill", "status.danger")
        case .idle: ("circle", nil)
        case .exited: ("circle.slash", nil)
        }
    }
}
