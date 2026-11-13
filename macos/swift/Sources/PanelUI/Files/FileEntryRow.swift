// One row of the tree (`row.rs`): the lead column with the chevron and the icon, the name in body text, a selected row in
// the primary's tint. A folder's icon and a file's come from the icon theme in force; a link has the link mark in place of
// its kind's icon. A note (an empty folder, "N more") is a muted line under the folder it is about. A file's name is the
// one thing the tab writes: Rename… turns it into a field.

import DesignSystem
import PanelModel
import SwiftUI

struct FileEntryRow: View {
    let store: FilesStore
    let row: FileRow
    @Environment(\.tokens) private var tokens

    var body: some View {
        if row.isEntry { entry.walkRow(row.path) } else { note }
    }

    private var selected: Bool { store.selected == row.path }
    private var muted: Color { selected ? tokens.sidebarRowSelectedMuted : tokens.textMuted }

    /// A folder's chevron starts the step; a file's icon sits under the icon of the folders beside it so their names line
    /// up, and a note is under the names.
    private func indent(extra: Int) -> CGFloat { PanelMetrics.inset + CGFloat(row.depth + extra) * Metric.indent }

    @ViewBuilder private var entry: some View {
        if store.renaming == row.path {
            // A field cannot sit inside a button: while it is renamed the row is plain.
            content.background(tokens.stateSelected, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        } else {
            PanelRow(selected: selected, action: { store.click(row) }) { _ in content }
                .contextMenu { FileRowMenu(store: store, row: row) }
                .help(row.label)
                .accessibilityLabel(spoken)
                .accessibilityValue({ if case let .folder(open) = row.kind { open ? panelText("files-spoken-expanded", "expanded") : panelText("files-spoken-collapsed", "collapsed") } else { "" } }())
        }
    }

    private var content: some View {
        HStack(spacing: PanelMetrics.gap) {
            if case let .folder(open) = row.kind { Disclosure(collapsed: !open) }
            icon
            if store.renaming == row.path { RenameField(store: store, row: row) } else { name }
            Spacer(minLength: 0)
        }
        .padding(.leading, indent(extra: row.isFolder ? 0 : 1)).padding(.trailing, PanelMetrics.inset)
        .frame(minHeight: Metric.row)
    }

    @ViewBuilder private var icon: some View {
        switch row.kind {
        case let .folder(open): FileIcon(kind: .folder(row.label, open: open), tint: muted)
        case .link: FileIcon(kind: .link, tint: muted)
        default: FileIcon(kind: .file(row.label), tint: muted)
        }
    }

    private var name: some View {
        Text(row.label).font(tokens.font(.body).weight(selected ? .semibold : .regular))
            .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary).lineLimit(1).truncationMode(.middle)
    }

    private var spoken: String {
        switch row.kind {
        case let .folder(open): open ? panelText("files-spoken-folder-open", ["name": .text(row.label)], "\(row.label), folder, expanded") : panelText("files-spoken-folder-closed", ["name": .text(row.label)], "\(row.label), folder, collapsed")
        case .link: panelText("files-spoken-link", ["name": .text(row.label)], "\(row.label), link")
        default: row.label
        }
    }

    /// A muted line about a folder: it is being read, empty, can't be read, or has more than is listed.
    private var note: some View {
        Text(words).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
            .padding(.leading, indent(extra: 2)).padding(.trailing, PanelMetrics.inset)
            .frame(maxWidth: .infinity, minHeight: Metric.row, alignment: .leading)
    }

    private var words: String {
        switch row.kind {
        case .reading: panelText("files-reading", "Reading\u{2026}")
        case .empty: panelText("files-empty", "Empty folder")
        case let .more(count, atLeast): atLeast ? panelText("files-more-at-least", ["count": .text(CountText.grouped(count))], "\(CountText.grouped(count))+ more") : panelText("files-more", ["count": .text(CountText.grouped(count))], "\(CountText.grouped(count)) more")
        case .unreadable(.gone): panelText("files-folder-gone", "This folder is gone.")
        case .unreadable(.denied): panelText("files-folder-denied", "Wormhole isn't allowed to read this folder.")
        case .unreadable(.other): panelText("files-folder-unreadable", "This folder can't be read.")
        default: ""
        }
    }
}
