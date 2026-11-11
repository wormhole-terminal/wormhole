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
        renaming = nil
        renameError = nil
    }

    public func commitRename(to name: String) {
        guard let root, let path = renaming else { return }
        let new = name.trimmingCharacters(in: .whitespaces)
        guard new != path.split(separator: "/").last.map(String.init) else { return cancelRename() }
        Task {
            switch await backend.rename(path, to: new, in: root) {
            case .success:
                let to = FileTree.parent(path).isEmpty ? new : FileTree.parent(path) + "/" + new
                tree.renamed(path, to: to)
                if selected == path { selected = to }
                if viewing == path { viewing = to }
                renaming = nil
                renameError = nil
                rows = tree.rows
                host?.notify("Renamed to \(new)")
                reload()
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
