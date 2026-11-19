// What a row or the open file hands to another app (`features/files/open.rs`). A folder's content is hostile, so the
// path is resolved inside the root by the core first, off the main actor: a file or folder swapped for a link that
// leads out since it was listed is handed to nobody, and it must still be what the row said it was.

import PanelModel

extension FilesStore {
    /// Opens the row's file or folder in the chosen app, or in the app with `bundle` (the Open in submenu).
    public func open(_ row: FileRow, inBundle bundle: String? = nil) {
        handOn(row.path, folder: row.isFolder) { [host] real in
            if let bundle { host?.open(real, inBundle: bundle) } else { host?.open(real, line: nil) }
        }
    }

    /// Shows the row's file or folder in Finder.
    public func reveal(_ row: FileRow) {
        handOn(row.path, folder: row.isFolder) { [host] in host?.reveal($0) }
    }

    /// Opens the file in the viewer in the chosen app.
    public func openViewed() {
        guard let viewing else { return }
        handOn(viewing, folder: false) { [host] in host?.open($0, line: nil) }
    }

    /// Shows the file in the viewer in Finder.
    public func revealViewed() {
        guard let viewing else { return }
        handOn(viewing, folder: false) { [host] in host?.reveal($0) }
    }

    private func handOn(_ path: String, folder: Bool, _ act: @escaping @MainActor (String) -> Void) {
        guard let root else { return }
        Task { [backend] in
            guard let real = await backend.resolve(path, in: root, folder: folder) else { return }
            act(real)
        }
    }
}
