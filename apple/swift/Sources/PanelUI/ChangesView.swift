// The Changes tab: the working tree's files, grouped Staged and Unstaged, and under them the selected file's
// diff. Arrow keys walk the files; the highlight slides, and the diff cross-fades in place.

import DesignSystem
import PanelModel
import SwiftUI

public struct ChangesView: View {
    let model: ChangesModel
    @State private var selection: ChangedFile.ID?
    @State private var cache = DiffCache()
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(model: ChangesModel, selection: ChangedFile.ID? = nil) {
        self.model = model
        _selection = State(initialValue: selection ?? model.listed.first?.id)
    }

    public var body: some View {
        if model.files.isEmpty {
            EmptyState(symbol: "checkmark.circle", title: "No changes", detail: "The working tree is clean.")
        } else {
            SelectableListSplit(selection: selection, label: "Changed files", step: step) {
                VStack(spacing: 0) {
                    FileListSection(title: "Staged", files: model.staged, selection: selection, space: space, select: choose)
                    FileListSection(title: "Unstaged", files: model.unstaged, selection: selection, space: space, select: choose)
                }
            } bottom: {
                detail
            }
        }
    }

    @ViewBuilder private var detail: some View {
        if let file = model.listed.first(where: { $0.id == selection }) {
            DiffPane(file: file, diff: cache.file(for: file.id, text: model.diffs[file.id]))
                .id(file.id)
                .transition(motion.content)
        }
    }

    private func choose(_ id: ChangedFile.ID) {
        withAnimation(motion.select) { selection = id }
    }

    private func step(_ delta: Int) {
        if let next = ListNavigation.move(in: model.listed.map(\.id), from: selection, by: delta) { choose(next) }
    }
}
