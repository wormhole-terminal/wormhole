// The menu of a row (`features/files/menus.rs`): open it in the editor, show it in Finder, copy its path, pin a folder
// or take it off the pins, and Rename… which turns the name into a field.

import PanelModel
import SwiftUI

struct FileRowMenu: View {
    let store: FilesStore
    let row: FileRow
    @Environment(\.panelHost) private var host

    var body: some View {
        if let editor = host?.editorName, !row.isFolder {
            Button(panelText("files-row-open-in-editor", "Open in \(editor)")) { host?.open(absolute, line: nil) }
        }
        Button(panelText("files-row-reveal", "Reveal in Finder")) { host?.reveal(absolute) }
        Button(panelText("files-row-copy-path", "Copy Path")) {
            host?.copyPath(absolute)
        }
        if row.isFolder {
            Divider()
            if store.isPinned(row.path) {
                Button(panelText("files-row-unpin-folder", "Unpin Folder")) { store.setPinned(row.path, false) }
            } else {
                Button(panelText("files-row-pin-folder", "Pin Folder")) { store.setPinned(row.path, true) }
            }
        }
        Divider()
        Button(panelText("files-row-rename", "Rename\u{2026}")) { store.startRename(row) }
    }

    private var absolute: String { store.root.map { store.join($0, row.path) } ?? row.path }
}
