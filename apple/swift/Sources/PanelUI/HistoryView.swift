// The History tab: the commits, newest first, and under them what the selected one changed. Arrow keys walk
// the commits; like Changes, the highlight slides and the detail cross-fades.

import DesignSystem
import PanelModel
import SwiftUI

public struct HistoryView: View {
    let model: HistoryModel
    let now: Date
    @State private var selection: Commit.ID?
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(model: HistoryModel, selection: Commit.ID? = nil, now: Date = Date()) {
        self.model = model
        self.now = now
        _selection = State(initialValue: selection ?? model.commits.first?.id)
    }

    public var body: some View {
        if model.commits.isEmpty {
            EmptyState(symbol: "clock", title: "No commits yet", detail: "Commits made in this repository show up here.")
        } else {
            SelectableListSplit(selection: selection, label: "Commits", maxTopShare: 0.55, step: step) {
                VStack(spacing: 0) {
                    ForEach(model.commits) { commit in
                        CommitRow(commit: commit, now: now, selected: commit.id == selection, space: space)
                            .padding(.horizontal, Space._1)
                            .onTapGesture { choose(commit.id) }
                    }
                }
            } bottom: {
                if let commit = model.commits.first(where: { $0.id == selection }) {
                    CommitDetail(commit: commit).id(commit.id).transition(motion.content)
                }
            }
        }
    }

    private func choose(_ id: Commit.ID) {
        withAnimation(motion.select) { selection = id }
    }

    private func step(_ delta: Int) {
        if let next = ListNavigation.move(in: model.commits.map(\.id), from: selection, by: delta) { choose(next) }
    }
}
