// The two side columns' own surfaces: the sidebar's (its title row over the list) and the right panel's (its
// tab bar over the tab). Each paints its translucent surface at the window's opacity over the blur, as at rest
// and while it slides, so the one choice cannot drift between them. On Liquid Glass the sidebar paints none:
// its cards float on the glass.

import SwiftUI
import SidebarUI
import WorkspaceModel

struct SidebarColumn: View {
    let state: WorkspaceState
    let facts: WindowFacts
    let send: @MainActor (WorkspaceAction) -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        VStack(spacing: 0) {
            SidebarTitleRow(facts: facts, send: send)
            SidebarView(state: state, send: send)
        }
        .background { if !look.glassOn { look.surface(colours.sidebar) } }
    }
}

struct PanelColumn: View {
    let tab: PanelTab
    var folder: String?
    let send: @MainActor (WorkspaceAction) -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        RightPanel(tab: tab, folder: folder, send: send)
            .background(look.surface(colours.panel))
    }
}
