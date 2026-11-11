// The menu of a row (`features/files/menus.rs`): open it in the editor, show it in Finder, copy its path, pin a folder
// or take it off the pins, and Rename… which turns the name into a field.

import AppKit
import PanelModel
import SwiftUI

struct FileRowMenu: View {
    let store: FilesStore
    let row: FileRow
    @Environment(\.panelHost) private var host

    var body: some View {
        openIn
        Button(panelText("files-row-reveal", "Reveal in Finder")) { host?.reveal(absolute) }
        Button(panelText("files-row-copy-path", "Copy Path")) {
            host?.copyPath(absolute)
        }
        Divider()
        Button(panelText("files-row-rename", "Rename\u{2026}")) { store.startRename(row) }
        if row.isFolder {
            Divider()
            if store.isPinned(row.path) {
                Button(panelText("files-row-unpin-folder", "Unpin Folder")) { store.setPinned(row.path, false) }
            } else {
                Button(panelText("files-row-pin-folder", "Pin Folder")) { store.setPinned(row.path, true) }
            }
        }
    }

    /// The apps Open in lists, the chosen one checked; a link is not opened (it may lead anywhere), so its submenu is off.
    private var openIn: some View {
        Menu(panelText("files-row-open-in", "Open in")) {
            // A file is offered only the editors Wormhole knows (and Finder); a folder every app (`may_open_in`).
            ForEach((host?.openApps ?? []).filter { row.isFolder || $0.isKnown }) { app in
                Toggle(isOn: Binding(get: { app.isChosen }, set: { _ in host?.open(absolute, inBundle: app.bundle) })) {
                    Label { Text(app.name) } icon: { if let icon = Self.icon(of: app.bundle) { Image(nsImage: icon) } }
                }
            }
        }
        .disabled(row.kind == .link)
    }

    /// The app's own icon at menu size.
    private static func icon(of bundle: String) -> NSImage? {
        guard let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle) else { return nil }
        let icon = NSWorkspace.shared.icon(forFile: url.path)
        icon.size = NSSize(width: 16, height: 16)
        return icon
    }

    private var absolute: String { store.root.map { store.join($0, row.path) } ?? row.path }
}
