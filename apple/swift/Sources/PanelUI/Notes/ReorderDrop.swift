// Dragging a next step or a note to another place in its list (`style::Reorder`, core `view::reorder`): the row under
// the pointer shows a line on the edge the dragged one would land on, above when dragging up and below when dragging
// down, and dropping moves it there. ⌥⌘↑ and ⌥⌘↓ do the same from the keyboard (`reorderKeys`).

import DesignSystem
import PanelModel
import SwiftUI

private struct ReorderDrop: DropDelegate {
    let id: UUID
    let ids: [UUID]
    @Binding var dragging: UUID?
    @Binding var target: UUID?
    let move: (Int, Int) -> Void

    func dropEntered(info: DropInfo) { target = id }
    func dropExited(info: DropInfo) { if target == id { target = nil } }
    func dropUpdated(info: DropInfo) -> DropProposal? { DropProposal(operation: .move) }

    func performDrop(info: DropInfo) -> Bool {
        defer { (dragging, target) = (nil, nil) }
        guard let dragged = dragging, let from = ids.firstIndex(of: dragged), let to = ids.firstIndex(of: id) else { return false }
        move(from, to)
        return true
    }
}

private struct Reorderable: ViewModifier {
    let id: UUID
    let ids: [UUID]
    @Binding var dragging: UUID?
    let move: (Int, Int) -> Void
    @State private var target: UUID?
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        content
            .overlay(alignment: edge == .above ? .top : .bottom) {
                if target == id, edge != nil { Rectangle().fill(tokens.primary).frame(height: 2).padding(.horizontal, PanelMetrics.inset) }
            }
            .onDrag {
                dragging = id
                return NSItemProvider(object: id.uuidString as NSString)
            }
            .onDrop(of: [.plainText], delegate: ReorderDrop(id: id, ids: ids, dragging: $dragging, target: $target, move: move))
    }

    private var edge: Reorder.Edge? {
        guard let dragging, let from = ids.firstIndex(of: dragging), let over = ids.firstIndex(of: id) else { return nil }
        return Reorder.edge(from: from, over: over)
    }
}

extension View {
    func reorderable(id: UUID, in ids: [UUID], dragging: Binding<UUID?>, move: @escaping (Int, Int) -> Void) -> some View {
        modifier(Reorderable(id: id, ids: ids, dragging: dragging, move: move))
    }

    /// ⌥⌘↑ and ⌥⌘↓ move the row being edited one place; `move` gets -1 or 1.
    func reorderKeys(_ move: @escaping (Int) -> Void) -> some View {
        onKeyPress(keys: [.upArrow, .downArrow], phases: .down) { press in
            guard press.modifiers == [.option, .command] else { return .ignored }
            move(press.key == .upArrow ? -1 : 1)
            return .handled
        }
    }
}
