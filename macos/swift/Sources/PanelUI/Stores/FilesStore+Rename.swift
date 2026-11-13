// Renaming a row in place (Rename… in its menu), the one thing the Files tab writes. The row's name becomes a field
// with the name selected and not its extension; ↩ commits, esc or a click elsewhere cancels. Refused, the field stays
// open with what was typed and the notice says why; done, the selection and an open viewer follow the new name.

import Foundation
import PanelModel

extension FilesStore {
    public func startRename(_ row: FileRow) {
        guard row.isEntry, !row.path.isEmpty else { return }
        renaming = row.path
        renameError = nil
    }

    public func cancelRename() {
        // While the rename runs the field stays: losing the keys then must not take it down (`rename.rs` `busy`).
        guard !renameBusy else { return }
        renaming = nil
        renameError = nil
    }

    public func commitRename(to name: String) {
        // One rename at a time: a second return would rename what is gone by then. What was typed goes to the core
        // untouched, which says what a usable name is.
        guard let root, let path = renaming, !renameBusy else { return }
        let new = name
        guard new != path.split(separator: "/").last.map(String.init) else { return cancelRename() }
        renameBusy = true
        Task {
            let answer = await backend.rename(path, to: new, in: root)
            renameBusy = false
            guard root == self.root else { return }
            switch answer {
            case .success:
                let to = FileTree.parent(path).isEmpty ? new : FileTree.parent(path) + "/" + new
                // The selection and the open file follow, also from inside a renamed folder.
                func moved(_ old: String) -> String {
                    old == path ? to : old.hasPrefix(path + "/") ? to + old.dropFirst(path.count) : old
                }
                tree.renamed(path, to: to)
                selected = selected.map(moved)
                viewing = viewing.map(moved)
                renaming = nil
                renameError = nil
                rows = tree.rows
                host?.notify(panelText("files-spoken-renamed", ["name": .text(new)], "Renamed to \(new)"))
                reload()
                // A renamed folder's pins moved in the settings file.
                pins = await backend.pins()
            case let .failure(failure):
                renameError = Self.words(failure)
                host?.notify(Self.words(failure))
                host?.failed(failure.panelFailed)
            }
        }
    }

    static func words(_ failure: RenameFailure) -> String {
        switch failure {
        case .name: panelText("notice-failed-rename-name", "Not renamed: that isn’t a usable name")
        case .exists: panelText("notice-failed-rename-exists", "Not renamed: something with that name exists already")
        case .denied: panelText("notice-failed-rename-denied", "Not renamed: Wormhole isn’t allowed to change it")
        case .gone: panelText("notice-failed-rename-gone", "Not renamed: it isn’t there any more")
        case .other: panelText("notice-failed-rename-failed", "Couldn’t rename it")
        }
    }
}

extension RenameFailure {
    var panelFailed: PanelFailed {
        switch self {
        case .name: .renameName
        case .exists: .renameExists
        case .denied: .renameDenied
        case .gone: .renameGone
        case .other: .renameFailed
        }
    }
}
