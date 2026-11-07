// The body of the window at one width: the columns by `PaneFrames`, the status bar, the floating toggles and
// the overlay. The slides are driven from here: a change of what the person wants animates its column, a change
// the window's width forces (the panel hidden for room) is a cut, as in the GPUI app.

import DesignSystem
import PaletteUI
import SwiftUI
import WorkspaceModel

struct WindowBody: View {
    let hub: WindowHub
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

    var state: WorkspaceState { hub.state }
    /// Terminals as tabs take the sidebar's place: its column stays shut however the person last left it.
    var sidebarWanted: Bool { state.sidebarVisible && !look.tabsOnTop }
    var send: @MainActor (WorkspaceAction) -> Void { hub.send }

    var body: some View {
        GeometryReader { proxy in
            let window = proxy.size.width
            let fit = ColumnFit(window: window, state: state, sidebarWanted: sidebarWanted, fallback: widths)
            let panelFits = fit.panelOpen
            VStack(spacing: 0) {
                SlideGeometry(sidebarShown: sidebarSlide.shown, panelShown: panelSlide.shown) { sidebar, panel in
                    panes(window: window, fit: fit, sidebarShown: sidebar, panelShown: panel)
                }
                if look.statusBar { statusBar(window: window) }
            }
            .overlay(alignment: .top) {
                WindowControls(
                    facts: facts, sidebarOpen: sidebarWanted, sidebarShown: sidebarSlide.shown,
                    panelOpen: panelFits, panelShown: panelSlide.shown,
                    panelWaiting: fit.panelWaiting, send: send)
            }
            .overlay { overlay }
            .overlay(alignment: .topLeading) {
                if let drag { ResizeReadout(text: drag.readout, pointer: drag.pointer) }
            }
            // The core fits the columns to the window's width, so it is told when that changes.
            .onChange(of: window, initial: true) { _, width in send(.setWindowWidth(Double(width))) }
            .onChange(of: Intent(sidebar: sidebarWanted, panelWanted: state.panelVisible, panelFits: panelFits), initial: true) {
                old, new in
                let wantedChanged = old.sidebar != new.sidebar || old.panelWanted != new.panelWanted
                let animated = animates && wantedChanged && drag == nil
                sidebarSlide.set(open: new.sidebar, animated: animated)
                panelSlide.set(open: new.panelWanted && new.panelFits, animated: animated)
            }
        }
    }

    private func panes(window: CGFloat, fit: ColumnFit, sidebarShown: CGFloat, panelShown: CGFloat) -> some View {
        let sidebarWidth = fit.sidebarWidth
        let sidebarColumn = PaneFrames.Column(
            width: sidebarWidth, shown: sidebarShown, inFlow: sidebarSlide.inFlow, sliding: sidebarSlide.sliding)
        let panelWidth = fit.panelWidth
        let panelColumn = PaneFrames.Column(
            width: panelWidth, shown: panelShown, inFlow: panelSlide.inFlow, sliding: panelSlide.sliding)
        let frames = PaneFrames(
            window: window, sidebar: sidebarColumn, panel: panelColumn, facts: facts,
            ruleWidth: look.columnDividers && !look.glassOn ? colours.dividerWidth : 0)
        return ZStack(alignment: .topLeading) {
            TerminalColumn(hub: hub, pads: frames.pads)
            .frame(width: frames.contentWidth)
            .offset(x: frames.carry)
            .frame(width: frames.slotVisible, alignment: .leading)
            .clipped()
            .offset(x: frames.slotX)

            SidebarColumn(hub: hub, facts: facts)
                .frame(width: sidebarWidth)
                .frame(width: frames.sidebarVisible, alignment: .leading)
                .clipped()
                .reachable(sidebarShown > 0)

            PanelColumn(hub: hub)
                .frame(width: panelWidth)
                .frame(width: frames.panelVisible, alignment: .trailing)
                .clipped()
                .reachable(panelShown > 0)
                .frame(width: window, alignment: .trailing)

            edges(frames, window: window, fit: fit)
        }
        .frame(width: window, alignment: .topLeading)
    }

    /// The hairlines and the handles over them, along a column that is in.
    @ViewBuilder private func edges(_ frames: PaneFrames, window: CGFloat, fit: ColumnFit) -> some View {
        if let edge = frames.sidebarEdge {
            if frames.ruleWidth > 0 { colours.divider.frame(width: frames.ruleWidth).offset(x: edge) }
            handle(.sidebar, at: edge, window: window, fit: fit)
        }
        if let edge = frames.panelEdge {
            if frames.ruleWidth > 0 { colours.divider.frame(width: frames.ruleWidth).offset(x: edge - frames.ruleWidth) }
            handle(.panel, at: edge, window: window, fit: fit)
        }
    }

    @ViewBuilder private var overlay: some View {
        switch state.overlay {
        case .palette: PaletteView(state: state, send: send, perform: { hub.actions.perform($0) }) { send(.showOverlay(nil)) }
        case .shortcuts: ShortcutSheetView { send(.showOverlay(nil)) }
        case nil: EmptyView()
        }
    }

    private func statusBar(window: CGFloat) -> some View {
        let grid = state.selected.flatMap { hub.terminals.existing($0) }?.renderer.gridSize
        return StatusBar(
            terminals: state.all, selected: state.selectedTerminal, size: grid.map { "\($0.cols)×\($0.rows)" },
            hosting: state.selected.flatMap { hub.store.hosting(of: $0) }, windowWidth: window
        ) { send(.select($0)) }
    }
}

extension View {
    /// A column that is clipped to nothing is out of reach of the pointer, the keys and VoiceOver.
    func reachable(_ reachable: Bool) -> some View {
        allowsHitTesting(reachable).accessibilityHidden(!reachable)
    }
}
