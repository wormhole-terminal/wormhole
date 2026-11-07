// What the hotkey quick panel lists, as data (`status_item/quick_panel/rows.rs`): without a query the status
// item's sections and Do; with one, the terminals and the things to do that it finds.

import Foundation

/// Something the panel's Do section offers.
public enum QuickPanelDo: Sendable, Equatable {
    /// A new terminal where the one in front is.
    case newTerminal(cwd: String?)
    case openFolder
    /// A project's action.
    case action(StatusSnapshot.Action)

    public var title: String {
        switch self {
        case .newTerminal: "New terminal"
        case .openFolder: "Open folder…"
        case .action(let action): "Run \(action.title)"
        }
    }

    public var detail: String? {
        switch self {
        case .newTerminal(let cwd): cwd.map { "in \(($0 as NSString).abbreviatingWithTildeInPath)" }
        case .openFolder: nil
        case .action(let action): "action · \(action.project)"
        }
    }

    /// The app's own key for it, shown as a key cap.
    public func keys(_ keymap: Keymap) -> String? {
        switch self {
        case .newTerminal: keymap.entry(.newTerminal)?.keys.last?.glyphText
        case .openFolder: keymap.key(.openFolder)?.glyphText
        case .action: nil
        }
    }

    public var command: AmbientCommand {
        switch self {
        case .newTerminal: .newTerminal
        case .openFolder: .openFolder
        case .action(let action): .runAction(root: action.root, id: action.id)
        }
    }

    func matches(_ query: String) -> Bool {
        let haystack = "\(title) \(detail ?? "")".lowercased()
        return query.split(whereSeparator: \.isWhitespace).allSatisfy { haystack.contains($0.lowercased()) }
    }
}

/// One line of the panel.
public enum QuickPanelRow: Sendable, Equatable, Identifiable {
    /// A section's label ("Needs input 2").
    case label(String, count: Int?)
    /// A quiet line where a section has nothing ("Nothing needs input").
    case note(String)
    case terminal(AmbientTerminal)
    case action(QuickPanelDo)

    public var id: String {
        switch self {
        case .label(let title, _): "label.\(title)"
        case .note(let text): "note.\(text)"
        case .terminal(let terminal): "terminal.\(terminal.tab)"
        case .action(let item): "do.\(item.title)"
        }
    }

    /// Whether ↑ ↓ stop on it.
    public var selectable: Bool {
        switch self {
        case .terminal, .action: true
        case .label, .note: false
        }
    }

    /// A terminal that waits for an answer: ⌘⏎ opens Reply on it.
    public var waiting: AmbientTerminal? {
        if case .terminal(let terminal) = self, terminal.state == .needsYou { terminal } else { nil }
    }

    /// What ⏎ or a click does.
    public var command: AmbientCommand? {
        switch self {
        case .terminal(let terminal): .openTab(terminal.tab)
        case .action(let item): item.command
        case .label, .note: nil
        }
    }
}

public enum QuickPanelRows {
    /// The panel's rows for `query` (empty: the sections).
    public static func rows(_ terminals: [AmbientTerminal], _ items: [QuickPanelDo], query: String) -> [QuickPanelRow] {
        let query = query.trimmingCharacters(in: .whitespaces)
        let sections = AmbientSections(terminals)
        var rows: [QuickPanelRow] = []
        func terminal(_ index: Int) -> QuickPanelRow { .terminal(terminals[index]) }
        if query.isEmpty {
            rows.append(.label("Needs input", count: sections.needsYou.count))
            if sections.needsYou.isEmpty { rows.append(.note("Nothing needs input")) }
            rows += sections.needsYou.map(terminal)
            for (title, list) in [("Working", sections.working), ("Recent", sections.finished)] where !list.isEmpty {
                rows.append(.label(title, count: list.count))
                rows += list.map(terminal)
            }
            rows.append(.label("Do", count: nil))
            return rows + items.map(QuickPanelRow.action)
        }
        let found = (sections.needsYou + sections.working + sections.finished + sections.other).filter { terminals[$0].matches(query) }
        if !found.isEmpty { rows += [.label("Terminals", count: found.count)] + found.map(terminal) }
        let things = items.filter { $0.matches(query) }
        if !things.isEmpty { rows += [.label("Do", count: nil)] + things.map(QuickPanelRow.action) }
        return rows.isEmpty ? [.note("Nothing matches")] : rows
    }
}
