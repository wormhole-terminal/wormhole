// What each specimen shows: the real views of the modules on sample data, never interactive (a specimen is for
// looking at, and a click inside one must not act on the sample), except the swatches that select their token.

import DesignSystem
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
            NotesSpecimen().frame(height: 360).clipped().allowsHitTesting(false)
        case "changes":
            ChangesSpecimen(opensFirst: false).frame(height: 300).clipped().allowsHitTesting(false)
        case "history":
            HistorySpecimen()
                .frame(height: 360).clipped().allowsHitTesting(false)
        case "diff":
            ChangesSpecimen(opensFirst: true)
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
