// The list's drag: the pointer's gesture reports where it is, the drop map says what letting go there would do, and the
// mark and the lifted item show it. Letting go sends the plan's action; a drag that ends where nothing would change, or
// is cancelled with Esc, does nothing.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct LiveDrag: Equatable {
    var item: DragItem
    var point: CGPoint
    var plan: DropPlan?
}

extension SidebarView {
    func dropMap(_ sections: [SidebarSection]) -> DropMap { DropMap(sections: sections, frames: frames) }

    func plan(_ item: DragItem, at point: CGPoint, _ sections: [SidebarSection]) -> DropPlan? {
        let map = dropMap(sections)
        let gap = SidebarMetrics(density: appearance.density, cards: appearance.cards).cardGap
        switch item {
        case .terminal(let id): return map.plan(dropping: id, at: point, gap: gap)
        case .group(let id): return map.plan(dropping: id, at: point, gap: gap)
        }
    }

    func dragMoved(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        drag = LiveDrag(item: item, point: point, plan: plan(item, at: point, sections))
    }

    func dragEnded(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        // Esc ended it already.
        guard drag != nil else { return }
        if let action = plan(item, at: point, sections)?.action { withAnimation(SidebarMotion.list) { send(action) } }
        drag = nil
    }

    /// The mark where the drop lands and the lifted item at the pointer.
    @ViewBuilder func dragLayer(_ sections: [SidebarSection]) -> some View {
        if let drag {
            let map = dropMap(sections)
            ZStack(alignment: .topLeading) {
                if let mark = drag.plan?.mark { DropMarkView(mark: mark, cards: map.cards) }
                DragGhost(terminal: ghostTerminal(drag.item), title: ghostTitle(drag.item))
                    .fixedSize().position(x: drag.point.x, y: drag.point.y)
            }
            .transition(.opacity)
        }
    }

    private func ghostTerminal(_ item: DragItem) -> TerminalSummary? {
        if case .terminal(let id) = item { state.terminals.first { $0.id == id } } else { nil }
    }

    private func ghostTitle(_ item: DragItem) -> String {
        switch item {
        case .terminal(let id): state.terminals.first { $0.id == id }?.title ?? ""
        case .group(let id): state.groups.first { $0.id == id }?.name ?? ""
        }
    }
}
