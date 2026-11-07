// The Changes tab: the list of changed files in cards, and under it the diff of the one opened. In the GPUI app the
// diff fills the terminal's surface; here the panel holds both, the list taking at most 45 % of the height. Arrow
// keys walk the files in the order the list shows them, a click opens one, esc closes the diff.

import DesignSystem
import PanelModel
import SwiftUI

public struct ChangesView: View {
    let model: ChangesModel
    @State private var layout = ChangeRows(tree: true)
    @State private var selection: ChangedFile.ID?
    @State private var cache = DiffCache()
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(model: ChangesModel, selection: ChangedFile.ID? = nil, tree: Bool = true) {
        self.model = model
        _selection = State(initialValue: selection)
        _layout = State(initialValue: ChangeRows(tree: tree))
    }

    public var body: some View {
        SelectableListSplit(selection: selection, hasDetail: current != nil, label: "Changed files", step: step) {
            ChangesListView(model: model, layout: $layout, selection: selection) { choose($0.id) }
                .environment(\.selectionSpace, space)
        } bottom: {
            if let file = current {
                DiffView(
                    file: file, diff: cache.file(for: file.id, text: model.diffs[file.id]),
                    position: order.firstIndex(of: file).map { ($0, order.count) }, step: step,
                    close: { choose(nil) })
            }
        }
    }

    private var order: [ChangedFile] { layout.order(model.files) }
    private var current: ChangedFile? { model.files.first { $0.id == selection } }

    private func choose(_ id: ChangedFile.ID?) { withAnimation(motion.select) { selection = id } }

    private func step(_ delta: Int) {
        let ids = order.map(\.id)
        if let next = ListNavigation.move(in: ids, from: selection, by: delta) { choose(next) }
    }
}
