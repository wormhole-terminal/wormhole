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
    /// Where in the row's card the pointer took hold of it, which the lifted picture keeps.
    var grab: CGPoint = .zero
}

extension EnvironmentValues {
    /// Where a terminal dragged out of the list is over the window, for the pane area to take its drop; the window's
    /// root sets it. Without one a row dragged out of the list lands nowhere.
    @Entry public var terminalDrag: TerminalDragPointer?
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
        // The edges between parts are held by the part the pointer was over, with the Motion feature on (`edge_memory`).
        let held: DropOver? = appearance.motionInForce ? drag?.aim.flatMap(\.over) : nil
        guard let over = lines.over(at: point.y, held: held, part: { dropPlanner?.dropPart(fraction: $0, previous: $1) ?? DropPart(fraction: $0) })
        else { return nil }
        let gap = SidebarMetrics(density: appearance.density, cards: appearance.cards).cardGap
        switch item {
        case .terminal(let id):
            guard let plan = dropPlanner?.dropPlan(dragged: id, over: over) else { return nil }
            var aim = DropAim(action: .dropTerminal(id, over: over), mark: lines.mark(for: plan, over: over, gap: gap))
            // Rows make room for a row that only moves, with the Motion feature on (`room.rs`).
            if appearance.motionInForce, let tables = RowMotion.tables {
                aim.room = lines.room(for: plan, dragged: id, splitOf: { target in state.all.first { $0.id == target }?.splitID }, tables: tables)
            }
            return aim
        case .group(let id):
            // The core says where the group would land (nil: nothing would change); the mark is that place.
            guard let landing = dropPlanner?.groupDropPlan(group: id, over: over) else { return nil }
            let plan = DropPlan.move(landing, leavesSplit: false, withMates: false)
            return DropAim(action: .dropGroup(id, over: over), mark: lines.mark(for: plan, over: over, gap: gap))
        }
    }

    /// A terminal dragged past the list's sides is over the panes (or the panel), not over a line of the list; a group never is.
    private func leftList(_ item: DragItem, _ point: CGPoint) -> Bool {
        guard case .terminal = item, viewport.width > 0 else { return false }
        return point.x < 0 || point.x > viewport.width
    }

    func dragMoved(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        guard !escape.ended else { return }
        escape.arm { cancelDrag() }
        outside = leftList(item, point)
        var grab = drag?.item == item ? drag?.grab ?? .zero : .zero
        if drag?.item != item, case .terminal(let id) = item, let frame = frames[.row(id)] { grab = CGPoint(x: point.x - frame.minX, y: point.y - frame.minY) }
        drag = LiveDrag(item: item, point: point, aim: outside ? nil : aim(item, at: point, sections), grab: grab)
        if case .terminal(let id) = item { if outside { terminalDrag?.move(id, to: windowPoint) } else { terminalDrag?.leave() } }
    }

    /// The pointer in the window's coordinates, as its second gesture reports it.
    func dragWindow(_ item: DragItem, _ point: CGPoint) {
        windowPoint = point
        if case .terminal(let id) = item { terminalDrag?.track(id, at: point) }
        if outside, case .terminal(let id) = item { terminalDrag?.move(id, to: point) }
    }

    /// Esc: the lifted row goes back, nothing is dropped, and the chip outside the window goes.
    func cancelDrag() {
        if case .terminal(let id)? = drag?.item { terminalDrag?.track(id, at: nil) }
        drag = nil
        outside = false
        terminalDrag?.leave()
        escape.disarm()
    }

    func dragEnded(_ item: DragItem, _ point: CGPoint, _ sections: [SidebarSection]) {
        defer { escape.finish() }
        // Esc ended it already.
        guard drag != nil else { return }
        defer {
            outside = false
            if case .terminal(let id) = item { terminalDrag?.track(id, at: nil) }
        }
        // Let go outside the window, the window decides (a window of its own, another window's list, or nothing).
        if case .terminal(let id) = item, terminalDrag?.tearOff?(id, windowPoint) == true {
            drag = nil
            return
        }
        if outside, case .terminal(let id) = item {
            terminalDrag?.release(id, at: windowPoint)
            drag = nil
            return
        }
        if let aimed = aim(item, at: point, sections) {
            let joins = if case .row = aimed.mark { true } else { false }
            withAnimation(reduce ? Motion.fade : joins ? SidebarMotion.joinSettle : SidebarMotion.list) { act(aimed.action) }
        }
        drag = nil
    }

    /// The mark where the drop lands and the lifted item at the pointer.
    @ViewBuilder func dragLayer(_ sections: [SidebarSection]) -> some View {
        if drag != nil || paneAim != nil {
            let lines = lines(sections)
            ZStack(alignment: .topLeading) {
                // While the rows make room the gap is the mark: no line is drawn.
                if let aim = drag?.aim ?? paneAim, let mark = aim.mark, !aim.room.isOpen { DropMarkView(mark: mark, lines: lines) }
                if let drag {
                    if case .terminal(let id) = drag.item, let row = sections.lazy.flatMap(\.rows).first(where: { $0.id == id }), let frame = frames[.row(id)] {
                        LiftedPicture(row: row, context: context(sections), drag: drag, width: frame.width, target: onto(drag.aim?.mark, lines))
                    } else {
                        DragGhost(terminal: ghostTerminal(drag.item), title: ghostTitle(drag.item))
                            .fixedSize().position(x: drag.point.x, y: drag.point.y)
                    }
                }
            }
            .transition(.opacity)
        }
    }

    /// The row a drop would make a split with, as the lifted picture's chip snaps onto it.
    private func onto(_ mark: DropMark?, _ lines: DropLines) -> CGRect? {
        if case .row(let id)? = mark { lines.rows[id] } else { nil }
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
