// A split drawn: each pane with its header, the dividers over their shared edges, the drop overlay, the
// whole surface. The tree and its geometry are the model's (`TabPanes`, the port of core's `view/panes`); the
// terminals are the window's, handed in through `content`. A zoomed pane fills the area and the others fade
// but stay alive underneath, so their terminals keep their place and the zoom reverses in place. Frames
// animate when the tree changes (a split opens, a drag lands), a fade under Reduce Motion.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

enum PaneSpace { static let name = "pane-area" }

public struct PaneArea<Content: View>: View {
    @Binding var panes: TabPanes
    let gaps: Bool
    let info: (PaneID) -> PaneInfo
    let actions: PaneActions
    let session: PaneDragSession
    /// A terminal dragged out of the sidebar's list, which the list reports as it goes (the list's drag is its own gesture).
    let sidebarDrag: TerminalDragPointer?
    @ViewBuilder let content: (PaneID) -> Content
    @State var drop = PaneDropState()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        panes: Binding<TabPanes>, gaps: Bool, session: PaneDragSession, sidebarDrag: TerminalDragPointer? = nil,
        actions: PaneActions = PaneActions(), info: @escaping (PaneID) -> PaneInfo, @ViewBuilder content: @escaping (PaneID) -> Content
    ) {
        (_panes, self.gaps, self.session, self.sidebarDrag, self.actions, self.info, self.content) =
            (panes, gaps, session, sidebarDrag, actions, info, content)
    }

    public var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            let origin = proxy.frame(in: .global).origin
            ZStack(alignment: .topLeading) {
                ForEach(panes.tree.rects(width: size.width, height: size.height), id: \.id) { pane in
                    frame(pane.id, rect: pane.rect, size: size)
                }
                PaneDividers(panes: $panes, size: size, gaps: gaps, session: session)
                PaneDropZones(panes: panes, state: drop, size: size, names: { info($0).title })
                if let over = sidebarDrag?.over { ghost(over, origin: origin) }
            }
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .coordinateSpace(.named(PaneSpace.name))
            .onDrop(of: [.plainText], delegate: PaneDropDelegate(state: $drop, panes: $panes, size: size, onDropped: actions.dropped))
            .animation(reduceMotion ? Motion.fade : Motion.layout, value: panes)
            .onChange(of: sidebarDrag?.over, initial: true) { _, over in follow(over, size: size, origin: origin) }
            .onChange(of: sidebarDrag?.released) { _, released in if let released { land(released, size: size, origin: origin) } }
        }
    }

    private func frame(_ id: PaneID, rect: PaneRect, size: CGSize) -> some View {
        let split = panes.isSplit
        let zoomed = panes.zoomed
        let shown = zoomed == nil || zoomed == id
        let place = zoomed == id ? PaneRect(0, 0, size.width, size.height) : inset(rect)
        let order = panes.panes
        let room = PaneRoom(size.width, size.height)
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
                    focus: { _ = panes.focus(id) }, zoom: { toggleZoom(id) }, close: { actions.close(id) }
                ) {
                    PaneMenuItems(id: id, panes: panes, room: room, actions: actions, focus: { _ = panes.focus(id) }, zoom: { toggleZoom(id) })
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
        _ = panes.focus(id)
        _ = panes.toggleZoom()
    }

    /// With gaps a pane is its own box: its cell less the gap on each side that has a neighbour (the gap
    /// belongs to the later pane of a pair, as GPUI's cells pad their start).
    private func inset(_ rect: PaneRect) -> PaneRect {
        guard gaps, panes.isSplit else { return rect }
        let (left, top) = (rect.x > 0.5 ? PaneMetrics.gap : 0, rect.y > 0.5 ? PaneMetrics.gap : 0)
        return PaneRect(rect.x + left, rect.y + top, rect.w - left, rect.h - top)
    }
}
