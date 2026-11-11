// The panes' side of the hub: the verbs on the layout the core keeps. The area's size goes to the core, which lays the panes
// out in it (`PanesState`), refuses a split or a drop that would leave a pane too small and decides every drop; the view
// draws what comes back.

import PanesModel
import PanesUI
import SwiftUI
import TerminalMetal
import WorkspaceModel

extension WindowHub {
    var paneActions: PaneActions {
        PaneActions(
            close: { [unowned self] in actions.closeTerminal($0) },
            split: { [unowned self] in split($0) },
            canSplit: { [unowned self] in canSplit($0) },
            leaveSplit: { [unowned self] in send(.leaveSplit($0)) },
            focus: { [unowned self] in send(.select($0)) },
            toggleZoom: { [unowned self] in toggleZoom() },
            dragDivider: { [unowned self] in send(.dragDivider(path: $0.path, index: $0.index, position: $1)) },
            equalize: { [unowned self] in send(.equalizeDivider(path: $0.path, index: $0.index)) },
            drop: { [unowned self] in drop($0, on: $1, zone: $2) },
            refusal: { [unowned self] in store.paneDropRefusal(dragged: $0.id, target: $1, zone: $2) },
            areaChanged: { [unowned self] in store.setPaneArea(width: $0.width, height: $0.height) })
    }

    /// ⌘D / ⇧⌘D, the split buttons and the pane menu: a new terminal beside the selected one, along `axis`.
    public func split(_ axis: SplitAxis) {
        guard let selected = state.selected else { return }
        send(axis == .row ? .splitRight(selected) : .splitDown(selected))
    }

    /// A terminal dropped on a pane's edge: a member of the split moves, any other joins it.
    private func drop(_ dragged: DraggedTerminal, on target: PaneID, zone: Zone) {
        send(state.panes.order.contains(dragged.id) ? .movePane(dragged.id, beside: target, zone: zone) : .joinSplit(dragged.id, with: target, zone: zone))
    }

    /// Whether the area has room for another pane that way: the core's answer.
    public func canSplit(_ axis: SplitAxis) -> Bool {
        guard let menu = state.layout?.menu else { return true }
        return axis == .row ? menu.canSplitRow : menu.canSplitColumn
    }

    /// ⇧⌘↩ and the pane header's zoom button.
    public func toggleZoom() { send(.toggleZoom) }

    /// ⌥⌘← → ↑ ↓: the neighbouring pane takes the keyboard.
    public func focusPane(_ direction: Direction) { send(.focusDirection(direction)) }
}

extension WindowHub {
    /// What the pane verbs depend on, for the menus: whether the selected terminal's split has several panes,
    /// whether one is zoomed.
    public var paneFacts: (isSplit: Bool, isZoomed: Bool) {
        let panes = state.panes
        return (panes.isSplit, panes.zoomed != nil)
    }
}

extension WindowHub {
    /// One line of how the panes stand, for the control folder's `state`.
    public var paneState: String {
        let panes = state.panes
        return "panes=\(panes.order.map(String.init).joined(separator: ",")) focused=\(panes.focused.map(String.init) ?? "none") " // text: control state line
            + "zoomed=\(panes.zoomed.map(String.init) ?? "none") cover=\(panel.cover.map { "\($0)" } ?? "none")" // text: control state line
    }
}

extension WindowHub {
    /// The cell and the strip beyond the grid of the one terminal that fills the area, in points, for a window that resizes
    /// in steps of a cell; nil in a split, whose panes share no grid.
    public var gridFacts: (cell: CGSize, spare: CGSize)? {
        guard !paneFacts.isSplit || paneFacts.isZoomed else { return nil }
        return state.selected.flatMap { terminals.existing($0) }?.cellAndSpare
    }
}
