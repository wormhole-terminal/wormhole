// The Files tree under the keys (`features/files/keys.rs`): every entry is a row the keys can be on, and the selected row is
// the cursor. → opens a folder (an open one goes to its first entry), ← closes it or goes to the parent, ↩ and Space open a file
// in the viewer and toggle a folder. With the viewer up, walking onto a file shows it; a folder is not toggled by walking.

import PanelModel

extension FilesStore: PanelWalking {
    public var walkRows: [WalkRow] {
        rows.filter(\.isEntry).map { row in
            WalkRow(id: row.path, level: row.depth, isOpen: row.isFolder ? tree.isOpen(row.path) : nil, name: row.name)
        }
    }

    public var cursor: String? {
        get { selected }
        set { selected = newValue }
    }

    public var following: Bool { viewing != nil }

    public func press(_ row: WalkRow, space: Bool, following: Bool) {
        guard let entry = rows.first(where: { $0.path == row.id }), !following || entry.kind == .file else { return }
        click(entry)
    }

    public func fold(_ row: WalkRow) { toggle(row.id) }
}
