// What each specimen shows: the real views of the modules on sample data, never interactive (a specimen is for
// looking at, and a click inside one must not act on the sample), except the swatches that select their token.

import DesignSystem
import PanelModel
import PanelUI
import SidebarUI
import SwiftUI
import WorkspaceModel

@MainActor
enum SpecimenContent {
    @ViewBuilder static func view(for id: String, context: GalleryContext) -> some View {
        switch id {
        case "sidebar":
            SidebarView(state: MockWorkspace.state, send: { _ in }).frame(height: 560).clipped().allowsHitTesting(false)
        case "notes":
            NotesView(model: .constant(PanelSamples.notes)).frame(height: 360).clipped().allowsHitTesting(false)
        case "changes":
            ChangesView(model: PanelSamples.changes).frame(height: 300).clipped().allowsHitTesting(false)
        case "history":
            HistoryView(model: PanelSamples.history, now: PanelSamples.now, expanded: PanelSamples.history.commits.first?.id)
                .frame(height: 360).clipped().allowsHitTesting(false)
        case "diff":
            ChangesView(model: PanelSamples.changes, selection: PanelSamples.changes.files.first?.id)
                .frame(height: 520).clipped().allowsHitTesting(false)
        case "chrome": ChromeSpecimen()
        case "shape": ShapeSpecimen()
        case "controls": ControlsSpecimen()
        case "palette": PaletteSpecimen()
        case "terminal": TerminalSpecimen(context: context)
        case "code": CodeSpecimen(context: context)
        default: TypeSpecimen()
        }
    }
}
