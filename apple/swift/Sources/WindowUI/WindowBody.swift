// The body of the window at one width: the columns by `PaneFrames`, the status bar, the floating toggles and
// the overlay. The slides are driven from here: a change of what the person wants animates its column, a change
// the window's width forces (the panel hidden for room) is a cut, as in the GPUI app.

import DesignSystem
import PaletteUI
import SwiftUI
import WorkspaceModel

struct WindowBody: View {
    let state: WorkspaceState
    let send: @MainActor (WorkspaceAction) -> Void
    let terminals: TerminalViews
    @Binding var widths: PaneLayout
    let facts: WindowFacts
    let sidebarSlide: ColumnSlide
    let panelSlide: ColumnSlide
    @Binding var drag: WorkspaceRootView.DragState?
    let animates: Bool
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    /// What decides whether a column slides: what is wanted, and whether the panel has room.
    private struct Intent: Equatable {
        var sidebar: Bool
        var panelWanted: Bool
        var panelFits: Bool
    }

    var body: some View {
        GeometryReader { proxy in
            let window = proxy.size.width
            let layout = layout(window)
            let panelFits = layout.panelVisible(window: window)
            VStack(spacing: 0) {
                SlideGeometry(sidebarShown: sidebarSlide.shown, panelShown: panelSlide.shown) { sidebar, panel in
                    panes(window: window, layout: layout, sidebarShown: sidebar, panelShown: panel)
                }
                if look.statusBar { statusBar(window: window) }
            }
            .overlay(alignment: .top) {
                WindowControls(
                    facts: facts, sidebarOpen: state.sidebarVisible, sidebarShown: sidebarSlide.shown,
                    panelOpen: panelFits, panelShown: panelSlide.shown,
                    panelWaiting: state.panelVisible && !panelFits, send: send)
            }
            .overlay { overlay }
            .overlay(alignment: .topLeading) {
                if let drag { ResizeReadout(text: drag.resize.readout(closing: drag.edge == .sidebar ? "Hide" : "Close"), pointer: drag.pointer) }
            }
            .onChange(of: Intent(sidebar: state.sidebarVisible, panelWanted: state.panelVisible, panelFits: panelFits), initial: true) {
                old, new in
                let wantedChanged = old.sidebar != new.sidebar || old.panelWanted != new.panelWanted
                let animated = animates && wantedChanged && drag == nil
                sidebarSlide.set(open: new.sidebar, animated: animated)
                panelSlide.set(open: new.panelWanted && new.panelFits, animated: animated)
            }
        }
    }

    private func layout(_ window: CGFloat) -> PaneLayout {
        var layout = widths
        layout.sidebarOpen = state.sidebarVisible
        layout.panelWanted = state.panelVisible
        return layout
    }

    private func panes(window: CGFloat, layout: PaneLayout, sidebarShown: CGFloat, panelShown: CGFloat) -> some View {
        let sidebarWidth = max(min(widths.sidebarWidth, window - PaneLayout.terminalMin), PaneLayout.sidebarRange.lowerBound)
        let sidebarColumn = PaneFrames.Column(
            width: sidebarWidth, shown: sidebarShown, inFlow: sidebarSlide.inFlow, sliding: sidebarSlide.sliding)
        let panelWidth = max(
            min(widths.panelWidth, window - sidebarColumn.inLayout - PaneLayout.terminalMin), PaneLayout.panelRange.lowerBound)
        let panelColumn = PaneFrames.Column(
            width: panelWidth, shown: panelShown, inFlow: panelSlide.inFlow, sliding: panelSlide.sliding)
        let frames = PaneFrames(
            window: window, sidebar: sidebarColumn, panel: panelColumn, facts: facts,
            ruleWidth: look.columnDividers && !look.glassOn ? colours.dividerWidth : 0)
        return ZStack(alignment: .topLeading) {
            TerminalColumn(
                terminal: state.selectedTerminal, selected: state.selected, ids: state.terminals.map(\.id),
                views: terminals, pads: frames.pads, showsState: !look.statusBar
            )
            .frame(width: frames.contentWidth)
            .offset(x: frames.carry)
            .frame(width: frames.slotVisible, alignment: .leading)
            .clipped()
            .offset(x: frames.slotX)

            SidebarColumn(state: state, facts: facts, send: send)
                .frame(width: sidebarWidth)
                .frame(width: frames.sidebarVisible, alignment: .leading)
                .clipped()
                .reachable(sidebarShown > 0)

            PanelColumn(tab: state.panelTab, send: send)
                .frame(width: panelWidth)
                .frame(width: frames.panelVisible, alignment: .trailing)
                .clipped()
                .reachable(panelShown > 0)
                .frame(width: window, alignment: .trailing)

            edges(frames, window: window, layout: layout)
        }
        .frame(width: window, alignment: .topLeading)
    }

    /// The hairlines and the handles over them, along a column that is in.
    @ViewBuilder private func edges(_ frames: PaneFrames, window: CGFloat, layout: PaneLayout) -> some View {
        if let edge = frames.sidebarEdge {
            if frames.ruleWidth > 0 { colours.divider.frame(width: frames.ruleWidth).offset(x: edge) }
            handle(.sidebar, at: edge, window: window)
        }
        if let edge = frames.panelEdge {
            if frames.ruleWidth > 0 { colours.divider.frame(width: frames.ruleWidth).offset(x: edge - frames.ruleWidth) }
            handle(.panel, at: edge, window: window)
        }
    }

    @ViewBuilder private var overlay: some View {
        switch state.overlay {
        case .palette: PaletteView(state: state, send: send) { send(.showOverlay(nil)) }
        case .shortcuts: ShortcutSheetView { send(.showOverlay(nil)) }
        case nil: EmptyView()
        }
    }

    private func statusBar(window: CGFloat) -> some View {
        let grid = state.selected.flatMap { terminals.existing($0) }?.renderer.gridSize
        return StatusBar(
            terminals: state.terminals, selected: state.selectedTerminal, size: grid.map { "\($0.cols)×\($0.rows)" }, windowWidth: window
        ) { send(.select($0)) }
    }
}

extension View {
    /// A column that is clipped to nothing is out of reach of the pointer, the keys and VoiceOver.
    func reachable(_ reachable: Bool) -> some View {
        allowsHitTesting(reachable).accessibilityHidden(!reachable)
    }
}
