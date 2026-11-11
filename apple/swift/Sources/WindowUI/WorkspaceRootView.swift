// The window's content: sidebar | terminal | right panel over the window's blur, the optional status bar under
// them, the two toggles floating over the title rows and the overlays over everything. The columns collapse by
// clip, not by removal, so their content keeps its layout while it slides and a running terminal view is never
// rebuilt (`SlideGeometry`, `PaneFrames`). The widths live here and only in memory: hiding a column and showing
// it again brings back the width it had.

import DesignSystem
import PaletteUI
import PanelUI
import SwiftUI
import WorkspaceModel

public struct WorkspaceRootView: View {
    let hub: WindowHub
    let look: WindowLook
    @State private var widths = PaneLayout()
    @State private var facts = WindowFacts()
    @State private var sidebarSlide: ColumnSlide
    @State private var panelSlide: ColumnSlide
    @State private var drag: DragState?
    @Environment(\.reduceMotion) private var reduceMotion

    /// A column being dragged: which, the pointer, what the readout says ("430 · max", "Close"), whether the column stays
    /// open at this pointer and how far the pointer is from the window's edge it hangs on.
    struct DragState: Equatable {
        var edge: PaneLayout.Edge
        var pointer: CGFloat
        var readout: String
        var open: Bool
        var extent: CGFloat
    }

    public init(hub: WindowHub, look: WindowLook = WindowLook()) {
        self.hub = hub
        self.look = look
        _sidebarSlide = State(initialValue: ColumnSlide(open: hub.state.sidebarVisible))
        _panelSlide = State(initialValue: ColumnSlide(open: hub.state.panelVisible))
    }

    public var body: some View {
        WindowBody(
            hub: hub, widths: $widths, facts: facts,
            sidebarSlide: sidebarSlide, panelSlide: panelSlide, drag: $drag, animates: !reduceMotion
        )
        .background(WindowBackdrop())
        .background(WindowReader(facts: $facts))
        .background(FocusHolder(focus: hub.focus))
        // A new window starts with the keys in its terminal, as the GPUI app's does (SwiftUI would give them to the first list).
        .task { hub.focus.focusTerminal() }
        // ⇥ and Esc from the lists and fields of SwiftUI; the holder and the terminal take their own (`WindowFocus`).
        .onKeyPress(keys: [.tab, .escape], phases: .down) { hub.focus.listKey($0) }
        .coordinateSpace(name: PaneSpace.name)
        .environment(look)
        .environment(\.codeFont, look.codeFont)
        .environment(\.tabWidth, hub.panel.preferences.tabWidth)
        .environment(hub.focus)
        .environment(\.windowFront, hub.focus.front)
        .themed()
        .ignoresSafeArea()
    }
}
