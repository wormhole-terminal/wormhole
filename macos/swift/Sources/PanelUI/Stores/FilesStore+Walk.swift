// The Files tree under the keys (`features/files/keys.rs`): every entry is a row the keys can be on, and the selected row is
// the cursor. → opens a folder (an open one goes to its first entry), ← closes it or goes to the parent, ↩ and Space open a file
// in the viewer and toggle a folder. With the viewer up, walking onto a file shows it; a folder is not toggled by walking.

import PanelModel

extension FilesStore: PanelWalking {
    /// The path button (`Line::Head`) is the first row, a band, so it is the tab's one ⇥ stop with the entries under it.
    public var walkRows: [WalkRow] {
        guard root != nil else { return [] }
        return [WalkRow(id: Self.headID, level: 0, isBand: true, name: pathMenu?.shownName ?? "")]
            + rows.filter(\.isEntry).map { row in
                WalkRow(id: row.path, level: 1 + row.depth, isOpen: row.isFolder ? tree.isOpen(row.path) : nil, name: row.label)
            }
    }

    public var cursor: String? {
        get { selected }
        set { selected = newValue }
    }

    public var following: Bool { viewing != nil }

    public func pressesBand(_ row: WalkRow) -> Bool { row.id == Self.headID }

    public func press(_ row: WalkRow, space: Bool, following: Bool) {
        // The path button: ↩ opens its menu (not while the keys only moved onto it with the viewer up).
        if row.id == Self.headID {
            if !following { pathMenuRequests += 1 }
            return
        }
        guard let entry = rows.first(where: { $0.path == row.id }), !following || entry.kind == .file else { return }
        click(entry)
    }

    public func fold(_ row: WalkRow) { if row.id != Self.headID { toggle(row.id) } }
}
