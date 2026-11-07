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
    let action: () -> Void
    @State private var width: CGFloat = 0
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelRow(selected: selected, action: action) { _ in
            HStack(spacing: PanelMetrics.gap) {
                Image(systemName: "doc").font(.system(size: Metric.glyph)).foregroundStyle(muted)
                    .frame(width: Metric.lead, height: Metric.lead)
                if let from = file.renamedFrom { renamed(from) }
                if file.moved {
                    Image(systemName: "folder.badge.gearshape").font(.system(size: 12)).foregroundStyle(muted)
                }
                Text(file.name)
                    .font(TextRole.body.font.weight(selected ? .semibold : .regular))
                    .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
                    .lineLimit(1).truncationMode(.middle)
                    .frame(maxWidth: max(width, 1) * PanelMetrics.fileNameShare, alignment: .leading)
                    .fixedSize(horizontal: true, vertical: false)
                if !inTree && !file.folder.isEmpty {
                    Text(file.folder).font(TextRole.caption.font).foregroundStyle(muted)
                        .lineLimit(1).truncationMode(.head)
                }
                Spacer(minLength: 0)
                FileLinesLabel(lines: file.lines, muted: muted)
                Text(file.kind.letter)
                    .font(TextRole.caption.font.weight(.bold))
                    .foregroundStyle(letterColor)
                    .frame(width: PanelMetrics.kindLetter, alignment: .trailing)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent)
            .padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
            .onGeometryChange(for: CGFloat.self, of: { $0.size.width - PanelMetrics.inset * 2 }) { width = $0 }
        }
        .help("\(file.path)\n\(file.about)")
        .accessibilityLabel("\(file.name), \(file.kind.title)")
    }

    private var muted: Color { selected ? tokens.sidebarRowSelectedMuted : tokens.textMuted }

    private func renamed(_ from: String) -> some View {
        HStack(spacing: Space._1) {
            Text(from).font(TextRole.caption.font).foregroundStyle(muted).lineLimit(1)
            Image(systemName: "arrow.right").font(.system(size: 12)).foregroundStyle(muted)
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
    let depth: Int
    let collapsed: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelRow(action: action) { _ in
            HStack(spacing: PanelMetrics.gap) {
                Disclosure(collapsed: collapsed)
                Image(systemName: collapsed ? "folder" : "folder.fill").font(.system(size: Metric.glyph))
                    .foregroundStyle(tokens.textMuted).frame(width: Metric.lead, height: Metric.lead)
                Text(label).font(TextRole.body.font).foregroundStyle(tokens.textMuted).lineLimit(1)
                Spacer(minLength: 0)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent)
            .padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
        }
        .accessibilityLabel(label)
        .accessibilityValue(collapsed ? "folded" : "open")
    }
}
