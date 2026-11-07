// The Files tab (`features/files/view.rs`): a card whose head is the path button and whose body is the tree of the
// terminal's folder, in the same scrolling column as the Changes rows, or a note where there is no folder yet. A file
// shows in the viewer, under the tree here or in the terminal's place when the window hosts it.

import DesignSystem
import PanelModel
import SwiftUI

public struct FilesView: View {
    @Bindable var store: FilesStore
    var presentation = DiffPresentation.panel
    @Namespace private var space
    @Environment(\.panelMotion) private var motion

    public init(store: FilesStore, presentation: DiffPresentation = .panel) {
        self.store = store
        self.presentation = presentation
    }

    public var body: some View {
        SelectableListSplit(
            selection: store.selected, hasDetail: presentation == .panel && store.viewing != nil, label: "Files", step: store.step,
            fold: store.fold(open:), press: store.press
        ) {
            tree.environment(\.selectionSpace, space)
        } bottom: {
            FileViewerView(store: store)
        }
    }

    private var tree: some View {
        PanelScroll {
            PanelCard {
                if store.root == nil {
                    EmptyNote(text: panelText("files-no-folder", "This terminal hasn't said which folder it is in yet."))
                } else {
                    FilesPathButton(store: store)
                    ForEach(store.rows) { row in
                        FileEntryRow(store: store, row: row).transition(motion.row)
                    }
                }
            }
        }
        .animation(motion.list, value: store.rows)
    }
}
