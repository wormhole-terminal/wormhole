// The Notes tab: the Next steps card above the Notes card, in the panel's scrolling column.

import PanelModel
import SwiftUI

public struct NotesView: View {
    let store: NotesStore

    public init(store: NotesStore) { self.store = store }

    public var body: some View {
        PanelScroll {
            StepsCard(store: store)
            NotesCard(store: store)
        }
    }
}
