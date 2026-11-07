// The Changes list: a card per section (`Changes`, `Untracked`), the first one's head carrying the compare
// picker, the view options and refresh. What replaces the list when nothing changed is a note in that card.
// Rows are `ChangeRows`' output; this only draws it and reports a click on a file.

import DesignSystem
import PanelModel
import SwiftUI

struct ChangesListView: View {
    let model: ChangesModel
    @Binding var layout: ChangeRows
    let selection: ChangedFile.ID?
    let open: (ChangedFile) -> Void
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelScroll {
            if model.files.isEmpty {
                PanelCard {
                    head(group: .all, count: 0, folded: false, first: true)
                    EmptyNote(text: "No changes. Working tree is clean.")
                }
            } else {
                ForEach(ChangeSection.cut(layout.build(model.files))) { section in
                    PanelCard {
                        if case let .band(group, count, folded, first) = section.band {
                            head(group: group, count: count, folded: folded, first: first)
                        }
                        ForEach(section.rows) { row in rowView(row).transition(motion.row) }
                    }
                    .transition(.opacity)
                }
            }
        }
        .animation(motion.list, value: layout.build(model.files))
    }

    private func head(group: ChangeGroup, count: Int, folded: Bool, first: Bool) -> some View {
        CardBand(
            title: group.title, folded: count > 0 ? folded : nil, count: count > 0 ? CountText.grouped(count) : nil,
            spoken: "\(group.title), \(count) \(count == 1 ? "file" : "files")",
            onToggle: { layout.folds[group] = !folded }
        ) {
            if first {
                BranchPicker(prefix: "vs", branch: model.compare, pick: { _ in })
                BandButton(symbol: "slider.horizontal.3", label: "View options") { layout.tree.toggle() }
                BandButton(symbol: "arrow.clockwise", label: "Refresh") {}
            }
        }
    }

    @ViewBuilder private func rowView(_ row: ChangeRow) -> some View {
        switch row {
        case let .file(file, depth, inTree):
            FileRowView(file: file, depth: depth, inTree: inTree, selected: file.id == selection) { open(file) }
        case let .directory(group, path, label, depth, collapsed):
            FolderRowView(label: label, depth: depth, collapsed: collapsed) {
                let key = "\(group.rawValue)\u{1f}\(path)"
                if collapsed { layout.collapsed.remove(key) } else { layout.collapsed.insert(key) }
            }
        case let .foldNote(text):
            Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
                .frame(maxWidth: .infinity, alignment: .leading)
        case .band, .spacer: EmptyView()
        }
    }

}
