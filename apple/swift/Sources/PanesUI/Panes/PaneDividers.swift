// The dividers over a split's shared edges, with their hover, drag and double-click. The core owns the tree and where a
// divider may go (it holds one where a pane would drop under its minimum); a drag only says where the pointer is.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct PaneDividers: View {
    let dividers: [PaneDividerState]
    let gaps: Bool
    let session: PaneDragSession
    let actions: PaneActions
    @State private var hot: PaneDividerState.ID?
    @State private var hoverTask: Task<Void, Never>?

    var body: some View {
        ForEach(dividers, id: \.id) { divider in
            let view = PaneDividerView(axis: divider.axis, gaps: gaps, look: look(of: divider.id))
            let (rect, row) = (divider.rect, divider.axis == .row)
            // With gaps the handle is the gap, at the start of the second pane's cell; else it straddles the edge.
            let along = divider.position + (gaps ? view.thickness / 2 : 0)
            view
                .frame(width: row ? nil : rect.w, height: row ? rect.h : nil)
                .position(x: row ? along : rect.x + rect.w / 2, y: row ? rect.y + rect.h / 2 : along)
                .onHover { hover(divider.id, $0) }
                .gesture(drag(divider))
                .simultaneousGesture(TapGesture(count: 2).onEnded { actions.equalize(divider.id) })
        }
    }

    private func look(of id: PaneDividerState.ID) -> DividerLook {
        if session.divider == id { return .dragging }
        return session.isActive ? .rest : (hot == id ? .hot : .rest)
    }

    private func drag(_ divider: PaneDividerState) -> some Gesture {
        DragGesture(minimumDistance: 1, coordinateSpace: .named(PaneSpace.name))
            .onChanged { value in
                guard !session.cancelled else { return }
                let along = divider.axis == .row ? value.location.x : value.location.y
                if !session.isActive {
                    let startAlong = divider.axis == .row ? value.startLocation.x : value.startLocation.y
                    session.begin(divider.id, grab: startAlong - divider.position) { actions.dragDivider(divider.id, divider.position) }
                }
                actions.dragDivider(divider.id, along - session.grab)
            }
            .onEnded { _ in session.end() }
    }

    /// Lights the line after a short rest, so crossing it on the way elsewhere doesn't flash it.
    private func hover(_ id: PaneDividerState.ID, _ inside: Bool) {
        hoverTask?.cancel()
        guard inside else { hot = nil; return }
        hoverTask = Task {
            try? await Task.sleep(for: PaneMetrics.hoverDelay)
            if !Task.isCancelled { hot = id }
        }
    }
}
