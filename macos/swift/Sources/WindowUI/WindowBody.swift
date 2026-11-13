// The body of the window at one width: the columns by `PaneFrames`, the status bar, the floating toggles and
// the overlay. The slides are driven from here: a change of what the person wants animates its column, a change
// the window's width forces (the panel hidden for room) is a cut, as in the GPUI app.

import DesignSystem
import PaletteUI
import PanesUI
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
    /// Where a column drag began, and whether Esc ended it (the pointer may still be down and moving).
    @State var dragBefore: DragBefore?
    @State var dragCancelled = false
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    struct DragBefore {
        var edge: PaneLayout.Edge
        var width: CGFloat
        var widths: PaneLayout
    }

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
            // Zen mode draws no panel and keeps what the window was asked for (`window::layout`): it is back as it was.
            let panelFits = fit.panelOpen && look.zen == nil
            let panelWanted = state.panelVisible && look.zen == nil
            VStack(spacing: 0) {
                SlideGeometry(sidebarShown: sidebarSlide.shown, panelShown: panelSlide.shown) { sidebar, panel in
                    panes(window: window, fit: fit, sidebarShown: sidebar, panelShown: panel)
                }
                if look.statusBar { statusBar(window: window).transition(.opacity) }
            }
            .overlay(alignment: .top) {
                WindowControls(
                    facts: facts, sidebarOpen: sidebarWanted, sidebarShown: look.iconAnimations ? sidebarSlide.shown : (sidebarWanted ? 1 : 0),
                    panelOpen: panelFits, panelShown: look.iconAnimations ? panelSlide.shown : (panelFits ? 1 : 0),
                    panelWaiting: fit.panelWaiting && look.zen == nil, leaveZen: look.zen == nil ? nil : { hub.actions.perform("toggleZenMode") },
                    send: { action in
                        send(action)
                        // A panel the button closed had the keys in a note or a list: the terminal takes them (`toggle_notes`).
                        if case .togglePanel = action, !hub.state.panelVisible { hub.focus.focusTerminal() }
                    })
            }
            // The status bar fades in and out (Reduce Motion cuts); the terminal resizes once, when it is gone.
            .animation(animates ? Motion.fade : nil, value: look.statusBar)
            .onAppear { hub.cancelColumnDrag = { cancelDrag() } }
            .overlay { overlay }
            // A window of another profile than the one Wormhole is in wears its profile's colour across the top (`profile_edge`).
            .overlay(alignment: .top) {
                if let odd = hub.actions.windowProfile?() {
                    Rectangle().fill(odd.colour ?? colours.primary).frame(height: Self.profileEdge).allowsHitTesting(false).accessibilityHidden(true)
                }
            }
            .overlay(alignment: .topLeading) {
                if let drag { ResizeReadout(text: drag.readout, pointer: drag.pointer, pointerY: drag.pointerY) }
            }
            // The core fits the columns to the window's width, so it is told when that changes.
            .onChange(of: window, initial: true) { _, width in send(.setWindowWidth(Double(width))) }
            // F6 and ⇥ skip what is not on screen.
            .onChange(
                of: WindowFocus.Layout(sidebar: sidebarWanted, panel: panelWanted && panelFits, statusBar: look.statusBar, tabsOnTop: look.tabsOnTop),
                initial: true
            ) { _, layout in hub.focus.layout = layout }
            .onChange(of: Intent(sidebar: sidebarWanted, panelWanted: panelWanted, panelFits: panelFits), initial: true) {
                old, new in
                let wantedChanged = old.sidebar != new.sidebar || old.panelWanted != new.panelWanted
                let animated = animates && wantedChanged && drag == nil
                sidebarSlide.set(open: new.sidebar, animated: animated, motion: look.columnMotion)
                panelSlide.set(open: new.panelWanted && new.panelFits, animated: animated, motion: look.columnMotion)
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
                .opacity(sidebarSlide.content)
                .reachable(sidebarShown > 0)

            PanelColumn(hub: hub)
                .frame(width: panelWidth)
                .frame(width: frames.panelVisible, alignment: .trailing)
                .clipped()
                .opacity(panelSlide.content)
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
        case .palette: PaletteView(state: state, send: send, sources: hub.actions.palette) { send(.showOverlay(nil)) }
        case .shortcuts:
            ShortcutSheetView(hidden: hub.actions.palette.hiddenActions(), hotkey: hub.actions.palette.hotkey(), project: hub.actions.palette.project()) { send(.showOverlay(nil)) }
        case nil: EmptyView()
        }
    }

    /// How tall the odd window's profile edge is (`PROFILE_EDGE_H`).
    private static let profileEdge: CGFloat = 3

    private func statusBar(window: CGFloat) -> some View {
        // The grid the program was told, redrawn when it changes (a column opening or closing resizes it after this is drawn).
        _ = hub.terminals.gridRevision
        let grid = state.selected.flatMap { hub.terminals.existing($0) }?.gridSize
        return StatusBar(
            terminals: hub.actions.everyTerminal?() ?? state.all, selected: state.selectedTerminal,
            program: state.selected.flatMap { hub.store.programName(of: $0) }, profile: hub.actions.windowProfile?(),
            size: grid.map { "\($0.cols)×\($0.rows)" },
            hosting: state.selected.flatMap { hub.store.hosting(of: $0) }, windowWidth: window,
            waited: { hub.store.waited(since: $0) },
            spokenAttention: { hub.store.spoken(attention: $0, since: $1) },
            spokenCount: { hub.store.spoken(count: $0, of: $1) }
        ) { hub.actions.showTerminal?($0) ?? send(.select($0)) }
    }
}

extension View {
    /// A column that is clipped to nothing is out of reach of the pointer, the keys and VoiceOver.
    func reachable(_ reachable: Bool) -> some View {
        allowsHitTesting(reachable).accessibilityHidden(!reachable)
    }
}
