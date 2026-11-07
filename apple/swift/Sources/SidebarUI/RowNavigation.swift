// Arrow-key movement through the rows that are on screen, as a pure function so it is tested without a view.

import WorkspaceModel

enum RowNavigation {
    /// Ids of the rows a person can see, top to bottom: folded sections contribute none.
    static func visibleIDs(_ sections: [SidebarSection]) -> [TerminalID] {
        sections.filter { !$0.isCollapsed }.flatMap { $0.rows.map(\.id) }
    }

    /// The row `step` places from `current`, stopping at the ends. With nothing selected, down picks the
    /// first row and up the last, as a native list does.
    static func neighbor(of current: TerminalID?, in ids: [TerminalID], step: Int) -> TerminalID? {
        guard !ids.isEmpty else { return nil }
        guard let current, let at = ids.firstIndex(of: current) else { return step > 0 ? ids.first : ids.last }
        return ids[min(max(at + step, 0), ids.count - 1)]
    }
}
