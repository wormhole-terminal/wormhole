// A pane's header dragged over the list (`sidebar/drop.rs`, `DragOrigin::PaneHeader`): it lands like a row would, by the core's
// drop plan (a move in the list, a join of another terminal's split), and let go where no row takes it, below the rows or on
// a row that changes nothing, the terminal leaves its split, unless it was let go on a terminal of that same split
// (`window/sidebar_drops.rs` `finish_terminal_drop`). The header is a system drag of `DraggedTerminal`'s text, so the list
// takes it as a drop target; the list's own rows are dragged by a gesture of their own (`SidebarView+Drag`).

import PanesModel
import SwiftUI
import UniformTypeIdentifiers
import WorkspaceModel

/// Which terminal a header drag carries, read when the drag enters (the text arrives asynchronously).
@MainActor final class PaneHeaderDragReader {
    var terminal: TerminalID?
}

struct PaneHeaderDrop: DropDelegate {
    let reader: PaneHeaderDragReader
    /// What letting go at this point (in the list's coordinates) would do; the list draws its mark.
    let aim: @MainActor (TerminalID, CGPoint) -> DropAim?
    let show: @MainActor (DropAim?) -> Void
    let finish: @MainActor (TerminalID, CGPoint) -> Void

    func validateDrop(info: DropInfo) -> Bool { info.hasItemsConforming(to: [.plainText]) }

    func dropEntered(info: DropInfo) {
        guard let provider = info.itemProviders(for: [.plainText]).first else { return }
        _ = provider.loadObject(ofClass: NSString.self) { text, _ in
            guard let dragged = (text as? String).flatMap(DraggedTerminal.init(encoded:)), dragged.origin == .paneHeader else { return }
            Task { @MainActor in reader.terminal = dragged.id }
        }
    }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        guard let id = reader.terminal else { return DropProposal(operation: .cancel) }
        show(aim(id, info.location))
        return DropProposal(operation: .move)
    }

    func dropExited(info: DropInfo) {
        show(nil)
        reader.terminal = nil
    }

    func performDrop(info: DropInfo) -> Bool {
        defer { show(nil); reader.terminal = nil }
        guard let id = reader.terminal else { return false }
        finish(id, info.location)
        return true
    }
}

/// What letting a pane's header go over the list does (`finish_terminal_drop`): the core's plan for the line under it, else the
/// terminal leaves its split, unless it was let go on a terminal of that same split.
enum PaneHeaderLanding: Equatable {
    case drop(DropOver)
    case leaveSplit
    case nothing

    static func decide(over: DropOver?, planned: Bool, inSplit: Bool, onSameSplit: Bool) -> PaneHeaderLanding {
        if let over, planned { return .drop(over) }
        return inSplit && !onSameSplit ? .leaveSplit : .nothing
    }
}

extension SidebarView {
    func paneHeaderDrop(_ sections: [SidebarSection]) -> PaneHeaderDrop {
        PaneHeaderDrop(
            reader: paneDragReader, aim: { aim(.terminal($0), at: $1, sections) }, show: { paneAim = $0 },
            finish: { id, point in
                let over = lines(sections).over(at: point.y)
                let split = state.all.first { $0.id == id }?.splitID
                var onSameSplit = false
                if case .row(let onto, _)? = over { onSameSplit = state.all.first { $0.id == onto }?.splitID == split }
                let planned = over.flatMap { dropPlanner?.dropPlan(dragged: id, over: $0) } != nil
                switch PaneHeaderLanding.decide(over: over, planned: planned, inSplit: split != nil, onSameSplit: onSameSplit) {
                case .drop(let over): act(.dropTerminal(id, over: over))
                case .leaveSplit: act(.leaveSplit(id))
                case .nothing: break
                }
            })
    }
}
