// How a row or a head takes part in a drag, as a value the list hands down: which item is lifted, and the two things a
// gesture reports (the pointer moved, the pointer let go). A drag is the pointer's own gesture in the list's coordinates
// rather than the system's pasteboard drag, so the lifted row, the mark where it lands and the settle are ours, and
// a drag that ends outside any target is simply nothing.

import SwiftUI
import WorkspaceModel

struct DragHooks {
    var item: DragItem?
    var moved: @MainActor (DragItem, CGPoint) -> Void
    var ended: @MainActor (DragItem, CGPoint) -> Void
    /// Where the pointer is in the window's coordinates, for a drag that leaves the list for the panes.
    var window: @MainActor (DragItem, CGPoint) -> Void = { _, _ in }

    func lifts(_ candidate: DragItem) -> Bool { item == candidate }

    /// Nothing is dragged and nothing can start: previews and tests.
    static let none = DragHooks(item: nil, moved: { _, _ in }, ended: { _, _ in })
}

enum DragLift {
    /// The lifted row's opacity where it stood (`DRAG_LIFT_OPACITY`).
    static let opacity = 0.4
    /// The least way the pointer moves before a press is a drag and not a click.
    static let slop: CGFloat = 6
    /// The coordinate space of the list's content, which frames and pointer positions are measured in.
    static let space = "sidebar-list"
}

extension View {
    func dragSource(_ item: DragItem, hooks: DragHooks) -> some View {
        simultaneousGesture(
            DragGesture(minimumDistance: DragLift.slop, coordinateSpace: .named(DragLift.space))
                .onChanged { hooks.moved(item, $0.location) }
                .onEnded { hooks.ended(item, $0.location) }
        )
        .simultaneousGesture(
            DragGesture(minimumDistance: DragLift.slop, coordinateSpace: .global).onChanged { hooks.window(item, $0.location) })
    }

    /// Reports where this line stands in the list, for the drop to find it.
    func dropFrame(_ slot: DropSlot) -> some View {
        background {
            GeometryReader { area in
                Color.clear.preference(key: DropFrames.self, value: [slot: area.frame(in: .named(DragLift.space))])
            }
        }
    }
}

struct DropFrames: PreferenceKey {
    static let defaultValue: [DropSlot: CGRect] = [:]
    static func reduce(value: inout [DropSlot: CGRect], nextValue: () -> [DropSlot: CGRect]) {
        value.merge(nextValue()) { _, new in new }
    }
}
