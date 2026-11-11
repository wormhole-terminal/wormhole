// The Changes tab: the list of changed files in cards, and under it the diff of the one opened. In the GPUI app the diff
// fills the terminal's surface (`.terminalArea`, hosted by the window); here by default the panel holds both, the list
// taking at most 45 % of the height. Arrow keys walk the files in the order the list shows them, a click opens one,
// esc closes the diff.

import DesignSystem
import PanelModel
import SwiftUI

public struct ChangesView: View {
    let store: ChangesStore
    var presentation = DiffPresentation.panel
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(store: ChangesStore, presentation: DiffPresentation = .panel) {
        self.store = store
        self.presentation = presentation
    }

    public var body: some View {
        SelectableListSplit(
            hasDetail: presentation == .panel && store.current != nil, label: "Changed files", walker: store
        ) {
            ChangesListView(store: store).environment(\.selectionSpace, space)
        } bottom: {
            ChangesDiffPane(store: store, headerInRow: false)
        }
    }
}

/// The diff of the file open in Changes, with the review bar under its rows.
struct ChangesDiffPane: View {
    let store: ChangesStore
    /// The window draws the header in the terminal's title row (`PanelCoverTitle`).
    let headerInRow: Bool

    var body: some View {
        if let file = store.current {
            DiffView(
                file: file, session: store.diff, position: store.position, step: store.step, close: store.close,
                review: store.canReview ? reviewing(file) : nil, showsHeader: !headerInRow
            )
            .alert("Copied the comments instead", isPresented: .init(get: { store.copiedInstead }, set: { store.copiedInstead = $0 })) {
                Button("OK") {}
            } message: {
                Text("The program in this terminal would run each line of a paste as a command, so the comments are on the clipboard. Paste them into your agent's prompt.")
            }
        }
    }

    private func reviewing(_ file: ChangedFile) -> DiffReview {
        DiffReview(
            review: store.review, put: { store.review.put($0) }, clear: { store.review.clear() },
            paste: store.pasteReview)
    }
}
