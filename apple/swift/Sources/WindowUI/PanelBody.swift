// The right panel's content: the Panel modules' views with their sample data. Kept in one file so the
// window knows the panel only through this seam; the real notes, changes and history arrive with the core.
// `PanelModel` has a tab type of its own (it knows nothing of the window), so it is matched by name here.

import PanelModel
import PanelUI
import SwiftUI

struct PanelBody: View {
    let tabName: String
    @State private var notes = PanelSamples.notes

    var body: some View {
        PanelContentView(
            tab: PanelModel.PanelTab(rawValue: tabName) ?? .notes, notes: $notes,
            changes: PanelSamples.changes, history: PanelSamples.history)
    }
}
