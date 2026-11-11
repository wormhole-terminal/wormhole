// A split drawn: each pane with its header, the dividers over their shared edges, the drop overlay, the whole surface. The
// layout is the core's (`PanesState`: where each pane is, which has the keyboard, which is zoomed); the terminals are the
// window's, handed in through `content`. A zoomed pane fills the area and the others fade but stay alive underneath, so their
// terminals keep their place and the zoom reverses in place. Frames animate when the layout changes (a split opens, a drag
// lands), a fade under Reduce Motion.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

enum PaneSpace { static let name = "pane-area" }

public struct PaneArea<Content: View>: View {
    let panes: PanesState
    let gaps: Bool
    let info: (PaneID) -> PaneInfo
    let actions: PaneActions
    let session: PaneDragSession
    /// A terminal dragged out of the sidebar's list, which the list reports as it goes (the list's drag is its own gesture).
    let sidebarDrag: TerminalDragPointer?
    @ViewBuilder let content: (PaneID) -> Content
    @State var drop = PaneDropState()
    @Environment(\.reduceMotion) private var reduceMotion

    public init(
        panes: PanesState, gaps: Bool, session: PaneDragSession, sidebarDrag: TerminalDragPointer? = nil,
        actions: PaneActions = PaneActions(), info: @escaping (PaneID) -> PaneInfo, @ViewBuilder content: @escaping (PaneID) -> Content
    ) {
        (self.panes, self.gaps, self.session, self.sidebarDrag, self.actions, self.info, self.content) =
            (panes, gaps, session, sidebarDrag, actions, info, content)
    }

    public var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            let origin = proxy.frame(in: .global).origin
            ZStack(alignment: .topLeading) {
                ForEach(panes.unzoomed, id: \.id) { pane in
                    frame(pane.id, rect: pane.rect, size: size)
                }
                PaneDividers(dividers: panes.dividers, gaps: gaps, session: session, actions: actions)
                PaneDropZones(panes: panes, state: drop, size: size, names: { info($0).title }, refusal: actions.refusal)
                if let over = sidebarDrag?.over { ghost(over, origin: origin) }
            }
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .coordinateSpace(.named(PaneSpace.name))
            .onDrop(of: [.plainText], delegate: PaneDropDelegate(state: $drop, panes: panes, actions: actions))
            .animation(reduceMotion ? Motion.fade : Motion.layout, value: panes)
            .onChange(of: size, initial: true) { _, size in actions.areaChanged(size) }
            .onChange(of: sidebarDrag?.over, initial: true) { _, over in follow(over, origin: origin) }
            .onChange(of: sidebarDrag?.released) { _, released in if let released { land(released, origin: origin) } }
        }
    }

    private func frame(_ id: PaneID, rect: PaneRect, size: CGSize) -> some View {
        let split = panes.isSplit
        let zoomed = panes.zoomed
        let shown = zoomed == nil || zoomed == id
        let place = zoomed == id ? PaneRect(0, 0, size.width, size.height) : inset(rect)
        let order = panes.order
        let focused = panes.focused == id
        let dragged = DraggedTerminal(id: id, origin: .paneHeader)
        return PaneFrame(
            focused: focused || !split, gaps: gaps && split,
            lifted: drop.isHovering && drop.dragged == dragged
        ) {
            if split {
                PaneHeader(
                    info: info(id), focused: focused, gaps: gaps, zoomed: zoomed == id, dragged: dragged,
                    position: (order.firstIndex(of: id) ?? 0) + 1, of: order.count,
                    focus: { actions.focus(id) }, zoom: { toggleZoom(id) }, close: { actions.close(id) }
                ) {
                    PaneMenuItems(id: id, panes: panes, actions: actions)
                }
            }
        } content: {
            content(id)
        }
        .placed(place)
        .opacity(shown ? 1 : 0)
        .allowsHitTesting(shown)
        .accessibilityHidden(!shown)
        .zIndex(zoomed == id ? 1 : 0)
        .transition(.opacity.combined(with: .scale(scale: 0.98)))
    }

    private func toggleZoom(_ id: PaneID) {
        if panes.focused != id { actions.focus(id) }
        actions.toggleZoom()
    }

    /// With gaps a pane is its own box: its cell less the gap on each side that has a neighbour (the gap
    /// belongs to the later pane of a pair, as GPUI's cells pad their start).
    private func inset(_ rect: PaneRect) -> PaneRect {
        guard gaps, panes.isSplit else { return rect }
        let (left, top) = (rect.x > 0.5 ? PaneMetrics.gap : 0, rect.y > 0.5 ? PaneMetrics.gap : 0)
        return PaneRect(rect.x + left, rect.y + top, rect.w - left, rect.h - top)
    }
}
