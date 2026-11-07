// The content of whichever tab is chosen. The window owns the strip and the frame; this swaps what is inside
// with a short cross-fade so changing tabs never cuts.

import PanelModel
import SwiftUI

public struct PanelContentView: View {
    let tab: PanelTab
    @Binding var notes: NotesModel
    let changes: ChangesModel
    let history: HistoryModel
    @Environment(\.panelMotion) private var motion

    public init(tab: PanelTab, notes: Binding<NotesModel>, changes: ChangesModel, history: HistoryModel) {
        self.tab = tab
        _notes = notes
        self.changes = changes
        self.history = history
    }

    public var body: some View {
        ZStack {
            switch tab {
            case .notes: NotesView(model: $notes).transition(motion.content)
            case .changes: ChangesView(model: changes).transition(motion.content)
            case .history: HistoryView(model: history).transition(motion.content)
            }
        }
        .animation(motion.tab, value: tab)
    }
}
