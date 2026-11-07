// What the status bar button shows (`menubar::model::status_icon`): a template glyph, filled while something
// works, and only the mark and the count carrying colour.

import Foundation

/// What the menu bar item shows (`config::MenuBarIcon`).
public enum MenuBarIconMode: String, Sendable, CaseIterable {
    /// The status dot and the number of terminals that need input.
    case statusCount = "status_count"
    case status
    case plain
}

/// What the dot says.
public enum StatusMark: Sendable, Equatable {
    case none
    /// The `agent.attention` dot.
    case needsYou
    /// The `agent.done` check.
    case finished
    /// The `status.danger` dot.
    case failed
}

public struct StatusIcon: Sendable, Equatable {
    public var state: ItemState
    public var mark: StatusMark
    /// How many need input ("9+" past nine), beside the glyph.
    public var count: String?
    public var tooltip: String
    /// The working glyph types its prompt out, only while working is the state.
    public var animate: Bool

    public init(_ terminals: [AmbientTerminal], mode: MenuBarIconMode, animate: Bool) {
        let counts = AmbientCounts(of: terminals)
        state = counts.itemState
        mark =
            switch (mode, state) {
            case (.plain, _): .none
            case (_, .needsYou): .needsYou
            case (_, .failed): .failed
            case (_, .finished): .finished
            default: .none
            }
        count = mode == .statusCount ? AmbientCounts.badge(counts.needsYou) : nil
        tooltip = counts.tooltip
        self.animate = animate && state == .working
    }
}
