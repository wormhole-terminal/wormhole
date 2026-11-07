// The list's drag: the pointer's gesture reports where it is, the lines say what it is over, the core says what letting go
// there would do, and the mark and the lifted item show it. Letting go sends the drop; a drag that ends where nothing would
// change, or is cancelled with Esc, does nothing.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct LiveDrag: Equatable {
    var item: DragItem
    var point: CGPoint
    var aim: DropAim?
}

extension EnvironmentValues {
    /// The store to ask what dropping a terminal here would do (the core's answer); the window's root sets it. Without
    /// one nothing would change, so a drag drops nothing.
    @Entry public var dropPlanner: WorkspaceStore?
}

extension SidebarView {
    func lines(_ sections: [SidebarSection]) -> DropLines { DropLines(sections: sections, frames: frames) }

    func aim(_ item: DragItem, at point: CGPoint, _ sections: [SidebarSection]) -> DropAim? {
        let lines = lines(sections)
        guard let over = lines.over(at: point.y) else { return nil }
        let gap = SidebarMetrics(density: appearance.density, cards: appearance.cards).cardGap
        switch item {
        case .terminal(let id):
            guard let plan = dropPlanner?.dropPlan(dragged: id, over: over) else { return nil }
            return DropAim(action: .dropTerminal(id, over: over), mark: lines.mark(for: plan, over: over, gap: gap))
        case .group(let id):
            return DropAim(action: .dropGroup(id, over: over), mark: lines.groupMark(over: over, at: point.y, gap: gap))
        }
    }

    func dragMoved(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        drag = LiveDrag(item: item, point: point, aim: aim(item, at: point, sections))
    }

    func dragEnded(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        // Esc ended it already.
        guard drag != nil else { return }
        if let action = aim(item, at: point, sections)?.action { withAnimation(SidebarMotion.list) { send(action) } }
        drag = nil
    }

    /// The mark where the drop lands and the lifted item at the pointer.
    @ViewBuilder func dragLayer(_ sections: [SidebarSection]) -> some View {
        if let drag {
            let lines = lines(sections)
            ZStack(alignment: .topLeading) {
                if let mark = drag.aim?.mark { DropMarkView(mark: mark, lines: lines) }
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
