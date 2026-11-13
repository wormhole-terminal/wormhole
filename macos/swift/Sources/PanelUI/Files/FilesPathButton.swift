// The button at the top of the card (`head.rs`): the shown folder's icon, its name and a chevron. It opens the path menu
// under itself like the menu a Finder window's title opens: the folder and each folder above it, the pins, then Pin,
// Choose Folder… and Follow Terminal.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

struct FilesPathButton: View {
    let store: FilesStore
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let menu = store.pathMenu, let root = store.root {
            Menu {
                content(menu)
            } label: {
                HStack(spacing: PanelMetrics.gap) {
                    FileIcon(kind: .folder(menu.shownName, open: false), tint: tokens.textMuted)
                    Text(menu.shownName).font(tokens.font(.label)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                    PanelIcon("chevron.down", size: 9).foregroundStyle(tokens.textMuted)
                    Spacer(minLength: 0)
                }
                .padding(.horizontal, PanelMetrics.inset).frame(height: Metric.band).frame(maxWidth: .infinity)
                .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
                .contentShape(Rectangle())
            }
            .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
            .walkRow(FilesStore.headID)
            .onHover { hovering = $0 }
            .help(root)
            .accessibilityLabel(panelText("files-spoken-path-button", ["name": .text(menu.shownName)], "\(menu.shownName), folder, opens the path menu"))
        }
    }

    @ViewBuilder private func content(_ menu: PathMenuModel) -> some View {
        ForEach(Array(menu.folders.enumerated()), id: \.element.id) { index, folder in
            if index == 0 {
                Toggle(isOn: .constant(true)) { Label { Text(folder.name) } icon: { Image(nsImage: icon(folder.path)) } }
            } else {
                Button { store.choose(folder.path) } label: { Label { Text(folder.name) } icon: { Image(nsImage: icon(folder.path)) } }
            }
        }
        if !menu.pinned.isEmpty {
            Divider()
            ForEach(menu.pinned) { pin in
                Button { store.choose(pin.path) } label: { Label { Text(pin.name) } icon: { Image(nsImage: icon(pin.path)) } }
            }
        }
        Divider()
        Button(menu.isPinned ? panelText("files-menu-unpin", ["name": .text(menu.shownName)], "Unpin \u{201c}\(menu.shownName)\u{201d}") : panelText("files-menu-pin", ["name": .text(menu.shownName)], "Pin \u{201c}\(menu.shownName)\u{201d}")) {
            store.setPinned("", !menu.isPinned)
        }
        Button(panelText("files-menu-choose-folder", "Choose Folder\u{2026}"), action: store.chooseFolder)
        if menu.canFollow { Button(panelText("files-menu-follow-terminal", "Follow Terminal"), action: store.followTerminal) }
    }

    /// Finder's own icon for a folder, unless it is not on this Mac's disk (a share, another volume): asking the system for
    /// that icon would touch the share, so it gets the generic folder's.
    private func icon(_ path: String) -> NSImage {
        let image = PathMenuModel.onBootVolume(path)
            ? NSWorkspace.shared.icon(forFile: path) : NSWorkspace.shared.icon(for: .folder)
        image.size = NSSize(width: 16, height: 16)
        return image
    }
}
