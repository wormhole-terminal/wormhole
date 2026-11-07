// The Changes list: a card per section (`Changes`, `Untracked`), the first one's head carrying the compare picker, the view
// options and refresh. What replaces the list when nothing changed is a note in that card. Rows are `ChangeRows`' output;
// this only draws it and reports a click on a file.

import DesignSystem
import PanelModel
import SwiftUI

struct ChangesListView: View {
    @Bindable var store: ChangesStore
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelScroll {
            switch store.phase {
            case .loading: PanelCard { head(group: .all, count: 0, folded: false, first: true); EmptyNote(text: "Reading changes\u{2026}") }
            case .notRepository: PanelCard { EmptyNote(text: "This folder isn't in a git repository.") }
            case let .failed(why): PanelCard { EmptyNote(text: why) }
            case .loaded: loaded
            }
        }
        .animation(motion.list, value: store.layout.build(store.files))
    }

    @ViewBuilder private var loaded: some View {
        if store.files.isEmpty {
            PanelCard {
                head(group: .all, count: 0, folded: false, first: true)
                EmptyNote(text: store.compare == nil ? "No changes. Working tree is clean." : "No differences from \(store.compare?.title ?? "").")
            }
        } else {
            ForEach(ChangeSection.cut(store.layout.build(store.files))) { section in
                PanelCard {
                    if case let .band(group, count, folded, first) = section.band { head(group: group, count: count, folded: folded, first: first) }
                    ForEach(section.rows) { row in rowView(row).transition(motion.row) }
                }
                .transition(.opacity)
            }
        }
    }

    private func head(group: ChangeGroup, count: Int, folded: Bool, first: Bool) -> some View {
        CardBand(
            title: group.title, folded: count > 0 ? folded : nil, count: count > 0 ? CountText.grouped(count) : nil,
            spoken: "\(group.title), \(count) \(count == 1 ? "file" : "files")",
            onToggle: { store.layout.folds[group] = !folded }
        ) {
            if first {
                RevPicker(prefix: "vs", selected: store.compare, revs: store.revs, headTitle: "HEAD", load: store.loadRevs, pick: store.setCompare)
                ViewOptionsMenu(layout: $store.layout)
                BandButton(symbol: "arrow.clockwise", label: "Refresh", action: store.refresh)
            }
        }
    }

    @ViewBuilder private func rowView(_ row: ChangeRow) -> some View {
        switch row {
        case let .file(file, depth, inTree):
            FileRowView(file: file, depth: depth, inTree: inTree, selected: file.id == store.selection) { store.open(file.id) }
        case let .directory(group, path, label, depth, collapsed):
            FolderRowView(label: label, depth: depth, collapsed: collapsed) {
                let key = "\(group.rawValue)\u{1f}\(path)"
                if collapsed { store.layout.collapsed.remove(key) } else { store.layout.collapsed.insert(key) }
            }
        case let .foldNote(text):
            Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
                .frame(maxWidth: .infinity, alignment: .leading)
        case .band, .spacer: EmptyView()
        }
    }
}
