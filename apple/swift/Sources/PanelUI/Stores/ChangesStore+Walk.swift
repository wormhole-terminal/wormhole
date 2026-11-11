// The Changes list under the keys (`features/git/changes/list.rs`): each section's band, its folders and files are the rows
// the keys can be on. ↩ and Space open a file's diff; on a folder they fold it, on a band only Space does. A diff that is open
// follows the keys from file to file. Marks (⌘-click, ⇧-click) are for the menu: they never open a diff.

import PanelModel

extension ChangesStore: PanelWalking {
    public var walkRows: [WalkRow] {
        layout.build(files).compactMap { row in
            switch row {
            case let .band(group, count, folded, _):
                WalkRow(id: row.id, level: 0, isOpen: count > 0 ? !folded : nil, isBand: true, name: group.title)
            case let .directory(_, _, label, depth, collapsed):
                WalkRow(id: row.id, level: 1 + depth, isOpen: !collapsed, name: label)
            case let .file(file, depth, _): WalkRow(id: row.id, level: 1 + depth, name: file.name)
            case .foldNote, .spacer: nil
            }
        }
    }

    public var following: Bool { selection != nil }

    public func press(_ row: WalkRow, space: Bool, following: Bool) {
        switch line(row) {
        case let .file(file, _, _): open(file.id)
        case .band, .directory: if !following { fold(row) }
        default: break
        }
    }

    public func fold(_ row: WalkRow) { line(row).map(toggle) }

    public func mark(_ row: WalkRow) { if case let .file(file, _, _) = line(row), file.submodule == nil { marked.insert(file.path) } }

    /// A click on a file: ⌘ marks or unmarks it, ⇧ marks every file between the last one clicked and it, a plain click opens
    /// its diff (and drops the marks).
    public func click(_ file: ChangedFile, command: Bool, shift: Bool) {
        // A submodule is never marked: ⌘-click opens it like any click.
        if command, file.submodule == nil {
            if !marked.insert(file.path).inserted { marked.remove(file.path) }
            anchor = file.path
        } else if shift, let from = anchor, let a = order.firstIndex(where: { $0.path == from }), let b = order.firstIndex(of: file) {
            marked.formUnion(order[min(a, b)...max(a, b)].map(\.path))
        } else {
            anchor = file.path
            open(file.id)
        }
    }

    /// What a right-click on `file` acts on: every marked file when it is one of them, else the file alone.
    public func menuPaths(for file: ChangedFile) -> [String] {
        marked.contains(file.path) ? order.map(\.path).filter(marked.contains) : [file.path]
    }

    /// A band folds or opens its section, a folder its place in the tree.
    public func toggle(_ row: ChangeRow) {
        switch row {
        case let .band(group, _, folded, _): layout.folds[group] = !folded
        case let .directory(group, path, _, _, collapsed):
            let key = "\(group.rawValue)\u{1f}\(path)"
            if collapsed { layout.collapsed.remove(key) } else { layout.collapsed.insert(key) }
        default: break
        }
    }

    private func line(_ row: WalkRow) -> ChangeRow? { layout.build(files).first { $0.id == row.id } }
}
