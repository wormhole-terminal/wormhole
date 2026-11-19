// Rows make room (`sidebar/motion/room.rs`): while a terminal is dragged by its row over the list, the lines it passes step aside
// on a spring to open a gap where it would land, the row it came from leaves the list, and no drop line is drawn. Where the gap
// is comes from the drop's own plan (the line it lands before) and how far each line steps is the core's (`room::make_room`,
// `RowMotionTables.roomPushes`); this keeps what is pushed and draws it. Every line is pushed inside its own layout box, so
// the frames the drop measures never move. Only with the Motion feature on; Reduce Motion shows the target at once.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct ListRoom: Equatable {
    /// How far each line is pushed down (negative: up).
    var pushes: [DropSlot: CGFloat] = [:]
    /// The dragged rows that are out of the list while a gap is open for them.
    var lifted: Set<TerminalID> = []

    var isOpen: Bool { !pushes.isEmpty }
}

extension EnvironmentValues {
    @Entry var listRoom = ListRoom()
}

extension DropLines {
    /// The room a drop of terminal `dragged` as `plan` says makes; none for a join or a landing beside its own place.
    func room(for plan: DropPlan, dragged: TerminalID, splitOf: (TerminalID) -> Int?, tables: RowMotionTables) -> ListRoom {
        guard case .move(let landing, _, _) = plan, let own = lines.firstIndex(where: { $0.kind == .row(dragged) }) else { return ListRoom() }
        // The dragged line and the lines of its split that stand beside it move together (`room::run_of`).
        let split = splitOf(dragged)
        func mate(_ at: Int) -> Bool {
            guard let split, case .row(let id) = lines[at].kind else { return false }
            return splitOf(id) == split
        }
        var (start, end) = (own, own + 1)
        while start > 0, mate(start - 1) { start -= 1 }
        while end < lines.count, mate(end) { end += 1 }
        // The line the gap opens before: the drop's own answer.
        let slot: Int?
        switch landing {
        case .top(let before):
            switch before {
            case .terminal(let id)?: slot = lines.firstIndex { $0.kind == .row(id) }
            case .group(let id)?: slot = lines.firstIndex { $0.kind == .head(id) }
            case nil: slot = lines.count
            }
        case .group(let group, let before):
            if let before {
                slot = lines.firstIndex { $0.kind == .row(before) }
            } else {
                let last = members[group]?.last.flatMap { id in lines.firstIndex { $0.kind == .row(id) } }
                slot = last.map { $0 + 1 } ?? lines.firstIndex { $0.kind == .head(group) }.map { $0 + 1 }
            }
        }
        guard let slot else { return ListRoom() }
        let pushes = tables.roomPushes(lines.map { (Double($0.frame.minY), Double($0.frame.height)) }, start..<end, slot)
        guard !pushes.isEmpty else { return ListRoom() }
        var room = ListRoom()
        for (at, by) in pushes {
            switch lines[at].kind {
            case .row(let id): room.pushes[.row(id)] = by
            case .head(let group): room.pushes[.head(group)] = by
            }
        }
        for at in start..<end { if case .row(let id) = lines[at].kind { room.lifted.insert(id) } }
        return room
    }
}

/// Reports where a line stands, and pushes it aside while a row is dragged past it.
struct DropFrame: ViewModifier {
    let slot: DropSlot
    @Environment(\.listRoom) private var room
    @Environment(\.reduceMotion) private var reduceMotion

    private var gone: Bool {
        if case .row(let id) = slot { room.lifted.contains(id) } else { false }
    }

    func body(content: Content) -> some View {
        // The offset comes before the measuring, which stays where the line is laid out.
        content
            .offset(y: room.pushes[slot] ?? 0)
            .opacity(gone ? 0 : 1)
            .animation(reduceMotion ? nil : SidebarMotion.list, value: room.pushes[slot] ?? 0)
            .background {
                GeometryReader { area in
                    Color.clear.preference(key: DropFrames.self, value: [slot: area.frame(in: .named(DragLift.space))])
                }
            }
    }
}
