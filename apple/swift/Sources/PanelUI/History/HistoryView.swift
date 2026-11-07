// The History tab (`features/git/history/view.rs`): one card, its head "History" with the branch picker and the count, then
// the log. Opening a commit selects its row and opens its details and its files under it; opening it again, or another,
// closes it. A file of a commit is the same row as in Changes, one level in, and a click on it opens its diff under the
// list (or in the terminal's place when the window hosts it). A log limited to some paths says so in a chip.

import DesignSystem
import Foundation
import PanelModel
import SwiftUI

public struct HistoryView: View {
    let store: HistoryStore
    var presentation = DiffPresentation.panel
    let now: Int
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(store: HistoryStore, presentation: DiffPresentation = .panel, now: Int = Int(Date().timeIntervalSince1970)) {
        self.store = store
        self.presentation = presentation
        self.now = now
    }

    public var body: some View {
        SelectableListSplit(
            selection: store.selection, hasDetail: presentation == .panel && store.current != nil, label: "Commits",
            step: store.step
        ) {
            HistoryListView(store: store, now: now).environment(\.selectionSpace, space)
        } bottom: {
            HistoryDiffPane(store: store, headerInRow: false)
        }
    }
}

/// The diff of the file open in the opened commit.
struct HistoryDiffPane: View {
    let store: HistoryStore
    let headerInRow: Bool

    var body: some View {
        if let file = store.current {
            DiffView(file: file, session: store.diff, position: store.position, step: store.step, close: store.close, showsHeader: !headerInRow)
        }
    }
}

struct HistoryListView: View {
    let store: HistoryStore
    let now: Int
    @Environment(\.panelMotion) private var motion
    @Environment(\.panelCopy) private var copy
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelScroll {
            PanelCard {
                CardBand(
                    title: "History", count: store.model.commits.isEmpty ? nil : CountText.grouped(store.model.commits.count),
                    spoken: "History, \(store.model.commits.count) commits"
                ) {
                    RevPicker(
                        selected: store.rev, revs: store.revs, headTitle: store.model.branch ?? "HEAD", load: store.loadRevs, pick: store.pick)
                }
                if !store.paths.isEmpty { filterChip }
                switch store.phase {
                case .loading: EmptyNote(text: "Reading the log\u{2026}")
                case .notRepository: EmptyNote(text: "This folder isn't in a git repository.")
                case let .failed(why): EmptyNote(text: why)
                case .loaded: commits
                }
            }
        }
    }

    @ViewBuilder private var commits: some View {
        if store.model.commits.isEmpty {
            EmptyNote(text: "No commits yet.")
        } else {
            ForEach(store.model.commits) { commit in
                CommitRowView(commit: commit, now: now, expanded: commit.id == store.expanded) {
                    withAnimation(motion.fold) { store.toggle(commit.id) }
                }
                .commitMenu(commit, openParent: openParent)
                if commit.id == store.expanded { details }
            }
        }
    }

    @ViewBuilder private var details: some View {
        if let commit = store.details {
            Group {
                CommitDetailsView(commit: commit, now: now, copy: { copy($0) }, open: openParent)
                ForEach(commit.files) { file in
                    FileRowView(file: file, depth: 1, selected: file.id == store.selection) { store.open(file.id) }
                }
            }
            .transition(motion.hunk)
        }
    }

    private func openParent(_ sha: String) {
        store.show(sha)
    }

    /// "File History": the chip that limits the log, with the way back to every commit.
    private var filterChip: some View {
        HStack(spacing: PanelMetrics.gap) {
            Image(systemName: "line.3.horizontal.decrease").font(.system(size: 12)).foregroundStyle(tokens.textMuted)
            Text(store.paths.count == 1 ? (store.paths[0].split(separator: "/").last.map(String.init) ?? store.paths[0]) : "\(store.paths.count) paths")
                .font(TextRole.caption.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
            Spacer(minLength: 0)
            BandButton(symbol: "xmark", label: panelText("history-filter-clear", "Show all commits"), inRow: true) { store.filter(by: []) }
        }
        .padding(.horizontal, PanelMetrics.inset).frame(minHeight: Metric.row)
    }
}
