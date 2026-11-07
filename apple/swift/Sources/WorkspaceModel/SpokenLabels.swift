// What VoiceOver says for a row and a group head, in the words and order of `wormhole-core`
// `view/sidebar/spoken.rs`: one label that carries the state, the detail, the selection, the place in
// its set and the shortcut, so the dot and keycap themselves are never read. English only for now.

import Foundation

extension SidebarListing {
    /// "sidebar polish, working, ~/Projects/app, branch main, 2 unread, pinned".
    public static func spokenLabel(for terminal: TerminalSummary) -> String {
        var parts = [terminal.title, terminal.activity.spokenWord]
        parts.append(contentsOf: [terminal.subtitle, terminal.branch.map { "branch \($0)" }].compactMap { $0 })
        if terminal.unread > 0 { parts.append("\(terminal.unread) unread") }
        if terminal.isPinned { parts.append("pinned") }
        return parts.joined(separator: ", ")
    }

    /// A row in its list: the label above, then "selected", "2 of 5" and "Command 2" (core's `row_label`).
    public static func spokenLabel(for row: SidebarRow, selected: Bool, position: Int, of count: Int) -> String {
        var parts = [spokenLabel(for: row.terminal)]
        if selected { parts.append("selected") }
        parts.append("\(position) of \(count)")
        if let number = row.number { parts.append("Command \(number)") }
        return parts.joined(separator: ", ")
    }

    /// A group's head: "Wormhole, 3 terminals, 1 need attention, collapsed, 2 of 4" (core's `band_label`).
    /// The attention count is said only while folded, as the dot on a folded head is all the eyes get of it.
    public static func spokenLabel(for section: SidebarSection, position: Int, of count: Int) -> String {
        var parts = [section.title ?? "Terminals"]
        parts.append(
            section.terminalCount == 0
                ? "no terminals" : "\(section.terminalCount) \(section.terminalCount == 1 ? "terminal" : "terminals")")
        if section.isCollapsed && section.alertCount > 0 { parts.append("\(section.alertCount) need attention") }
        parts.append(section.isCollapsed ? "collapsed" : "expanded")
        parts.append("\(position) of \(count)")
        return parts.joined(separator: ", ")
    }
}

extension Activity {
    /// core's `state_word`.
    fileprivate var spokenWord: String {
        switch self {
        case .idle: "idle"
        case .working: "working"
        case .needsInput: "needs input"
        case .finished: "finished"
        case .failed: "failed"
        }
    }
}
