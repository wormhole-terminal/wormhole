// The window's content: sidebar | terminal | right panel over the window's blur, the optional status bar under
// them, the two toggles floating over the title rows and the overlays over everything. The columns collapse by
// clip, not by removal, so their content keeps its layout while it slides and a running terminal view is never
// rebuilt (`SlideGeometry`, `PaneFrames`). The widths live here and only in memory: hiding a column and showing
// it again brings back the width it had.

import DesignSystem
import PaletteUI
import SwiftUI
import WorkspaceModel

public struct WorkspaceRootView: View {
    let state: WorkspaceState
    let send: @MainActor (WorkspaceAction) -> Void
    let terminals: TerminalViews
    let look: WindowLook
    @State private var widths = PaneLayout()
    @State private var facts = WindowFacts()
    @State private var sidebarSlide: ColumnSlide
    @State private var panelSlide: ColumnSlide
    @State private var drag: DragState?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    /// A column being dragged: which, the pointer, what the readout says and whether letting go closes it.
    struct DragState: Equatable {
        var edge: PaneLayout.Edge
        var pointer: CGFloat
        var resize: ColumnResize
    }

    public init(
        state: WorkspaceState, look: WindowLook = WindowLook(),
        send: @escaping @MainActor (WorkspaceAction) -> Void, terminals: TerminalViews
    ) {
        self.state = state
        self.look = look
        self.send = send
        self.terminals = terminals
        _sidebarSlide = State(initialValue: ColumnSlide(open: state.sidebarVisible))
        _panelSlide = State(initialValue: ColumnSlide(open: state.panelVisible))
    }

    public var body: some View {
        WindowBody(
            state: state, send: send, terminals: terminals, widths: $widths, facts: facts,
            sidebarSlide: sidebarSlide, panelSlide: panelSlide, drag: $drag, animates: !reduceMotion
        )
        .background(WindowBackdrop())
        .background(WindowReader(facts: $facts))
        .coordinateSpace(name: PaneSpace.name)
        .environment(look)
        .themed()
        .ignoresSafeArea()
    }
}
