// The dividers over a split's shared edges, with their hover, drag and double-click.

import DesignSystem
import PanesModel
import SwiftUI

struct PaneDividers: View {
    @Binding var panes: TabPanes
    let size: CGSize
    let gaps: Bool
    let session: PaneDragSession
    @State private var hot: DividerID?
    @State private var hoverTask: Task<Void, Never>?

    var body: some View {
        let room = PaneRoom(size.width, size.height)
        ForEach(panes.dividers(width: size.width, height: size.height), id: \.id) { divider in
            let view = PaneDividerView(axis: divider.axis, gaps: gaps, look: look(of: divider.id))
            let (rect, row) = (divider.rect, divider.axis == .row)
            // With gaps the handle is the gap, at the start of the second pane's cell; else it straddles the edge.
            let along = divider.position + (gaps ? view.thickness / 2 : 0)
            view
                .frame(width: row ? nil : rect.w, height: row ? rect.h : nil)
                .position(x: row ? along : rect.x + rect.w / 2, y: row ? rect.y + rect.h / 2 : along)
                .onHover { hover(divider.id, $0) }
                .gesture(drag(divider, room: room))
                .simultaneousGesture(TapGesture(count: 2).onEnded { _ = panes.equalize(divider.id) })
        }
    }

    private func look(of id: DividerID) -> DividerLook {
        if session.divider == id { return .dragging }
        return session.isActive ? .rest : (hot == id ? .hot : .rest)
    }

    private func drag(_ divider: PaneDivider, room: PaneRoom) -> some Gesture {
        DragGesture(minimumDistance: 1, coordinateSpace: .named(PaneSpace.name))
            .onChanged { value in
                let along = divider.axis == .row ? value.location.x : value.location.y
                if !session.isActive {
                    let startAlong = divider.axis == .row ? value.startLocation.x : value.startLocation.y
                    session.begin(divider.id, panes: panes, grab: startAlong - divider.position) { panes = $0 }
                }
                _ = panes.drag(divider.id, to: along - session.grab, room: room)
            }
            .onEnded { _ in session.end() }
    }

    /// Lights the line after a short rest, so crossing it on the way elsewhere doesn't flash it.
    private func hover(_ id: DividerID, _ inside: Bool) {
        hoverTask?.cancel()
        guard inside else { hot = nil; return }
        hoverTask = Task {
            try? await Task.sleep(for: PaneMetrics.hoverDelay)
            if !Task.isCancelled { hot = id }
        }
    }
}
