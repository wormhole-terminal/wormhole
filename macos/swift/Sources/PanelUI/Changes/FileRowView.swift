// One changed file as the Changes and History lists draw it (`style::file_row`): its icon in the lead column,
// the name (at most 55 % of the row, cut in the middle), its folder in muted caption text cut at the front,
// then on the right the line counts and the kind letter in its status colour. A renamed file shows where it
// came from before the new name, a moved one a mark. Each tree level steps in by 24. Selected: the tint, the
// name semibold in the tint's text colours.

import DesignSystem
import PanelModel
import SwiftUI

struct FileRowView: View {
    let file: ChangedFile
    var depth = 0
    var inTree = false
    var selected = false
    /// Marked for the menu (⌘-click): the fill and the tint of a selected row, without being the open one.
    var marked = false
    /// What the menu acts on (the marked files); the file alone by default. `onDisk` nil asks the disk.
    var menu: [String]?
    var onDisk: Bool?
    let action: () -> Void
    @State private var width: CGFloat = 0
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root

    var body: some View {
        PanelRow(selected: selected, marked: marked, action: action) { hovering in
            HStack(spacing: PanelMetrics.gap) {
                FileIcon(kind: file.isFolder ? .folder(file.name, open: false) : .file(file.path), tint: muted)
                if let from = file.renamedFrom { renamed(from) }
                if file.moved {
                    PanelIcon("folder.badge.plus", size: 12).foregroundStyle(muted)
                }
                Text(file.name)
                    .font(tokens.font(.body).weight(lit ? .semibold : .regular))
                    .foregroundStyle(lit ? tokens.sidebarRowSelectedText : tokens.textPrimary)
                    .lineLimit(1).truncationMode(.middle)
                    .frame(maxWidth: max(width, 1) * PanelMetrics.fileNameShare, alignment: .leading)
                    .fixedSize(horizontal: true, vertical: false)
                if !inTree && !file.folder.isEmpty {
                    Text(file.folder).font(tokens.font(.caption)).foregroundStyle(muted)
                        .lineLimit(1).truncationMode(.head)
                }
                Spacer(minLength: 0)
                // The icon that opens the file in the chosen editor, over the row while the pointer is (a deleted file has none).
                if hovering, onDisk ?? (file.kind != .deleted), let open = host?.openTooltip(line: nil) {
                    BandButton(symbol: "arrow.up.forward.square", label: open, inRow: true) {
                        // A path core refuses (a link out of the repository) is not handed on.
                        if let path = root.flatMap({ RepoFiles.resolve(root: $0, relative: file.path) }) { host?.open(path, line: nil) }
                    }
                }
                FileLinesLabel(lines: file.lines, muted: muted)
                Text(file.kind.letter)
                    .font(tokens.font(.caption).weight(.bold))
                    .foregroundStyle(letterColor)
                    .frame(width: PanelMetrics.kindLetter, alignment: .trailing)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent)
            .padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
            .onGeometryChange(for: CGFloat.self, of: { $0.size.width - PanelMetrics.inset * 2 }) { width = $0 }
        }
        .marked("file.row")
        .help("\(file.path)\n\(file.about)")
        .pathMenu(menu ?? [file.path], onDisk: onDisk)
        .accessibilityLabel("\(file.name), \(file.kind.title)")
    }

    private var lit: Bool { selected || marked }
    private var muted: Color { lit ? tokens.sidebarRowSelectedMuted : tokens.textMuted }

    private func renamed(_ from: String) -> some View {
        HStack(spacing: Space._1) {
            Text(from).font(tokens.font(.caption)).foregroundStyle(muted).lineLimit(1)
            PanelIcon("arrow.right", size: 12).foregroundStyle(muted)
        }
    }

    private var letterColor: Color {
        switch file.kind {
        case .modified: tokens.statusWarningText
        case .added, .untracked: tokens.statusSuccessText
        case .deleted, .conflicted: tokens.statusDangerText
        case .renamed: tokens.sidebarRowSelectedText
        }
    }
}

/// A folder of the tree: the chevron in the lead column, then the folder's icon and its name in muted text
/// (a chain of single folders is one row, "crates/wormhole/src"). Folds on a click.
struct FolderRowView: View {
    let label: String
    /// Relative to the repository: what its menu acts on.
    let path: String
    let depth: Int
    let collapsed: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelRow(action: action) { _ in
            HStack(spacing: PanelMetrics.gap) {
                Disclosure(collapsed: collapsed)
                FileIcon(kind: .folder(label, open: !collapsed), tint: tokens.textMuted)
                Text(label).font(tokens.font(.body)).foregroundStyle(tokens.textMuted).lineLimit(1)
                Spacer(minLength: 0)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent)
            .padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
        }
        .pathMenu([path])
        .accessibilityLabel(label)
        .accessibilityValue(collapsed ? panelText("panel-spoken-folded", "folded") : panelText("panel-spoken-open", "open"))
    }
}
