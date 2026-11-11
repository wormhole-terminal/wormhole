// The Dock icon's menu (`menubar::model::dock_menu`): waiting and running terminals under their headers, then
// New Terminal and New Window; macOS adds its own items below. And the badge on the icon's tile.

import Foundation
import WordsModel

public enum DockMenu {
    public static func build(_ terminals: [AmbientTerminal], now: Date) -> [MenuNode] {
        let sections = AmbientSections(terminals)
        var nodes: [MenuNode] = []
        let titles = [
            (Words.text("palette-state-needs-input", fallback: "Needs input"), sections.needsYou),
            (Words.text("palette-state-working", fallback: "Working"), sections.working),
        ]
        for (title, list) in titles where !list.isEmpty {
            nodes.append(.header(title))
            for index in list.prefix(StatusMenu.maxSessions / 2) {
                let terminal = terminals[index]
                let waited = AmbientWording.took(max(0, now.timeIntervalSince(terminal.since))) ?? "now"
                nodes.append(.item(MenuItem(.ambient(.openTab(terminal.tab)), "\(terminal.name)   \(waited)")))
            }
        }
        if !nodes.isEmpty { nodes.append(.separator) }
        return nodes + [
            .item(MenuItem(.ambient(.newTerminal), Words.text("platform-menu-new-terminal", fallback: "New Terminal"))),
            .item(MenuItem(.ambient(.newWindow), Words.text("platform-menu-new-window", fallback: "New Window"))),
        ]
    }
}

/// What the Dock badge counts (`config::DockBadge`).
public enum DockBadgeMode: String, Sendable, CaseIterable {
    case off
    case needsYou = "needs_you"
    /// Terminals that need input and agents that finished while you were away.
    case needsYouDone = "needs_you_done"

    /// The Dock's count stops here ("999+", `view::limits::COUNT_BADGE_MAX`).
    static let badgeMax = 999

    /// The badge's label: nil clears it. Working never badges (`notify::dock_badge`).
    public func label(for counts: AmbientCounts) -> String? {
        let n =
            switch self {
            case .off: 0
            case .needsYou: counts.needsYou
            case .needsYouDone: counts.needsYou + counts.results
            }
        return n > 0 ? (n > Self.badgeMax ? "\(Self.badgeMax)+" : String(n)) : nil
    }
}
