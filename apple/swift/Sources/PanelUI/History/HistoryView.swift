// The History tab (`features/git/history/view.rs`): one card, its head "History" with the branch picker and the count, then
// the log. Opening a commit selects its row and opens its details and its files under it; opening it again, or another,
// closes it. A file of a commit is the same row as in Changes, one level in, and a click on it opens its diff under the
// list (or in the terminal's place when the window hosts it). A log limited to some paths says so in a chip.

import DesignSystem
import Foundation
import PanelModel
import SwiftUI
import WordsModel

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
            hasDetail: presentation == .panel && store.current != nil, label: panelText("history-list-label", "Commits"), walker: store
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
    @Environment(\.tokens) private var tokens

    var body: some View {
        PanelScroll {
            PanelCard {
                CardBand(
                    title: panelText("history-title", "History"), count: store.model.commits.isEmpty ? nil : CountText.grouped(store.model.commits.count),
                    spoken: panelText("history-spoken-list", ["count": .whole(store.model.commits.count)], "History, \(String(store.model.commits.count)) commits")
                ) {
                    RevPicker(
                        selected: store.rev, revs: store.revs, headTitle: "HEAD", load: store.loadRevs, pick: store.pick) // text: HEAD is git's name for the current commit
                }
                if !store.paths.isEmpty { filterChip }
                switch store.phase {
                case .loading: EmptyNote(text: panelText("history-reading", "Reading history\u{2026}"))
                case .notRepository: EmptyNote(text: panelText("git-not-repository", "This terminal's folder isn't inside a git repository."))
                case let .failed(why): GitProblemView(problem: GitProblem(message: why, tab: .history), retry: store.refresh)
                case .loaded: commits
                }
            }
        }
    }

    @ViewBuilder private var commits: some View {
        if store.model.commits.isEmpty {
            EmptyView()
        } else {
            ForEach(store.model.commits) { commit in
                CommitRowView(commit: commit, now: now, expanded: commit.id == store.expanded) {
                    withAnimation(motion.fold) { store.toggle(commit.id) }
                }
                .commitMenu(commit, store: store)
                .walkRow("commit-" + commit.id)
                if commit.id == store.expanded { details(of: commit) }
            }
        }
    }

    @ViewBuilder private func details(of commit: Commit) -> some View {
        Group {
            CommitDetailsView(
                commit: commit, details: store.details, error: store.detailsError, store: store,
                listed: Set(store.model.commits.map(\.sha)), open: openParent)
                .walkRow("details-" + commit.id)
            if let files = store.details?.files {
                if files.isEmpty { EmptyNote(text: panelText("history-no-files", "No file changes.")) }
                ForEach(files) { file in
                    if file.submodule != nil {
                        SubmoduleRowView(file: file, depth: 1, selected: file.id == store.selection) { store.open(file.id) }
                            .walkRow("file-" + file.id)
                    } else {
                        FileRowView(file: file, depth: 1, selected: file.id == store.selection) { store.open(file.id) }
                            .walkRow("file-" + file.id)
                    }
                }
            } else if store.detailsError == nil {
                EmptyNote(text: panelText("history-loading-files", "Loading files\u{2026}"))
            }
        }
        .transition(motion.hunk)
    }

    private func openParent(_ sha: String) {
        store.show(sha)
    }

    /// "File History": the pill that limits the log (the file's name or "N paths") with an x, and the way back to every commit
    /// on a click (`history/view.rs`, tooltip "Show all commits").
    private var filterChip: some View {
        let first = store.paths[0].split(separator: "/").last.map(String.init) ?? store.paths[0]
        let name = Words.text(
            "history-filter", ["count": .whole(store.paths.count), "name": .text(first)],
            fallback: store.paths.count == 1 ? first : "\(store.paths.count) paths")
        return HStack(spacing: PanelMetrics.gap) {
            Button { store.filter(by: []) } label: {
                HStack(spacing: Space._1) {
                    Text(name).font(TextRole.caption.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                    PanelIcon("xmark", size: 12).foregroundStyle(tokens.textMuted)
                }
                .padding(.horizontal, Space._2).frame(height: Metric.controlSmall)
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain).help(panelText("history-filter-clear", "Show all commits"))
            .accessibilityLabel(panelText("history-filter-clear", "Show all commits"))
            Spacer(minLength: 0)
        }
        .padding(.horizontal, PanelMetrics.inset).frame(minHeight: Metric.row)
    }
}
