// The sidebar's list as data: which rows, in which order, under which heading. Pure, so it is tested
// here and the view only draws it. The rules follow `wormhole-core` `view/sidebar` (pins, groups, search, ⌘1–9).

import Foundation

public struct SidebarRow: Sendable, Equatable, Identifiable {
    public var id: TerminalID { terminal.id }
    public var terminal: TerminalSummary
    /// ⌘1–⌘9 shortcut number, when the row has one.
    public var number: Int?
}

public struct SidebarSection: Sendable, Equatable, Identifiable {
    public var id: String
    public var title: String?  // nil: the ungrouped run
    public var isCollapsed: Bool
    public var rows: [SidebarRow]
}

public enum SidebarListing {
    /// First cut: every terminal in one untitled section. The model agent replaces this with the real rules.
    public static func sections(for state: WorkspaceState) -> [SidebarSection] {
        let rows = state.terminals.enumerated().map { SidebarRow(terminal: $1, number: $0 < 9 ? $0 + 1 : nil) }
        return [SidebarSection(id: "all", title: nil, isCollapsed: false, rows: rows)]
    }
}
