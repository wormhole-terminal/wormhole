// The History tab (`features/git/history/view.rs`): one card, its head "History" with the branch picker and the
// count, then the log. Opening a commit selects its row and opens its details and its files under it; opening it
// again, or another, closes it. A file of a commit is the same row as in Changes, one level in.

import DesignSystem
import PanelModel
import SwiftUI

public struct HistoryView: View {
    let model: HistoryModel
    let now: Int
    @State private var expanded: Commit.ID?
    @Environment(\.panelMotion) private var motion
    @Environment(\.panelCopy) private var copy

    public init(model: HistoryModel, now: Int = PanelSamples.now, expanded: Commit.ID? = nil) {
        self.model = model
        self.now = now
        _expanded = State(initialValue: expanded)
    }

    public var body: some View {
        PanelScroll {
            PanelCard {
                CardBand(
                    title: "History", count: model.commits.isEmpty ? nil : CountText.grouped(model.commits.count),
                    spoken: "History, \(model.commits.count) commits"
                ) { BranchPicker(branch: model.branch, pick: { _ in }) }
                if model.commits.isEmpty {
                    EmptyNote(text: "No commits yet.")
                } else {
                    ForEach(model.commits) { commit in
                        CommitRowView(commit: commit, now: now, expanded: commit.id == expanded) {
                            withAnimation(motion.fold) { expanded = expanded == commit.id ? nil : commit.id }
                        }
                        if commit.id == expanded { details(of: commit) }
                    }
                }
            }
        }
    }

    @ViewBuilder private func details(of commit: Commit) -> some View {
        Group {
            CommitDetailsView(commit: commit, now: now, copy: { copy($0) }, open: { sha in
                if let parent = model.commits.first(where: { $0.sha.hasPrefix(sha.prefix(7)) }) { expanded = parent.id }
            })
            ForEach(commit.files) { file in FileRowView(file: file, depth: 1) {} }
        }
        .transition(motion.hunk)
    }
}
