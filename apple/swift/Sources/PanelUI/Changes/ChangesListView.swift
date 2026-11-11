// The Changes list: a card per section (`Changes`, `Untracked`), the first one's head carrying the compare picker, the view
// options and refresh. What replaces the list when nothing changed is a note in that card. Rows are `ChangeRows`' output;
// this only draws it and reports a click on a file.

import DesignSystem
import PanelModel
import AppKit
import SwiftUI

struct ChangesListView: View {
    @Bindable var store: ChangesStore
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelScroll {
            switch store.phase {
            case .loading: PanelCard { head(group: .all, count: 0, folded: false, first: true); EmptyNote(text: panelText("changes-reading", "Reading git status\u{2026}")) }
            case .notRepository: PanelCard { EmptyNote(text: panelText("git-not-repository", "This terminal's folder isn't inside a git repository.")) }
            case let .failed(why): PanelCard { GitProblemView(problem: GitProblem(message: why, tab: .changes), retry: store.refresh) }
            case .loaded: loaded
            }
        }
        .animation(motion.list, value: store.layout.build(store.files))
    }

    @ViewBuilder private var loaded: some View {
        if let message = store.refreshError { StatusLine(text: panelText("changes-refresh-failed", "Couldn't refresh: \(message.split(separator: "\n").first.map(String.init) ?? message)"), danger: true) }
        if let branch = store.compareMissing { StatusLine(text: panelText("changes-compare-missing", "\(branch) no longer exists \u{00B7} showing vs HEAD"), danger: false) }
        if store.files.isEmpty {
            PanelCard {
                head(group: .all, count: 0, folded: false, first: true)
                EmptyNote(text: panelText("changes-clean", "No changes. Working tree is clean."))
            }
        } else {
            ForEach(ChangeSection.cut(store.layout.build(store.files))) { section in
                PanelCard {
                    if case let .band(group, count, folded, first) = section.band { head(group: group, count: count, folded: folded, first: first).walkRow(section.band.id) }
                    ForEach(section.rows) { row in rowView(row).walkRow(row.id).transition(motion.row) }
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
            let selected = file.id == store.selection
            let marked = store.marked.contains(file.path)
            let click = {
                let flags = NSApp.currentEvent?.modifierFlags ?? []
                store.click(file, command: flags.contains(.command), shift: flags.contains(.shift))
            }
            if file.submodule != nil {
                SubmoduleRowView(file: file, depth: depth, inTree: inTree, selected: selected, action: click)
            } else {
                FileRowView(
                    file: file, depth: depth, inTree: inTree, selected: selected, marked: marked, menu: store.menuPaths(for: file),
                    onDisk: file.kind != .deleted, action: click)
            }
        case let .directory(_, path, label, depth, collapsed):
            FolderRowView(label: label, path: path, depth: depth, collapsed: collapsed) { store.toggle(row) }
        case let .foldNote(text):
            Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2)
                .frame(maxWidth: .infinity, alignment: .leading)
        case .band, .spacer: EmptyView()
        }
    }
}
