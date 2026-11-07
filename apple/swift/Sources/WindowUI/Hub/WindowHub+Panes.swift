// The panes' side of the hub: the layout on screen as a binding the pane area edits, and the verbs on it.

import PanesModel
import PanesUI
import SwiftUI
import TerminalMetal
import WorkspaceModel

extension WindowHub {
    /// The panes of the selected terminal's split. Edits the area makes (a drop, a divider, a click) come back
    /// as workspace actions through `PaneLayouts.store`.
    var panesBinding: Binding<TabPanes> {
        Binding(
            get: { self.layouts.panes(in: self.state) ?? .single(0) },
            set: { self.commit($0) })
    }

    var paneActions: PaneActions {
        PaneActions(
            close: { [unowned self] in actions.closeTerminal($0) },
            split: { [unowned self] in split($0) },
            leaveSplit: { [unowned self] in send(.leaveSplit($0)) })
    }

    /// ⌘D / ⇧⌘D, the split buttons and the pane menu.
    public func split(_ axis: SplitAxis) {
        guard let selected = state.selected, var panes = layouts.panes(in: state), panes.canSplit(axis, room: PaneRoom(0, 0))
        else { return }
        // The core makes the terminal beside this one, in its split, and selects it; where it sits on screen is ours.
        guard store.send(.splitRight(selected)) == nil, let new = state.selected, new != selected, panes.split(axis, new: new) else { return }
        layouts.save(panes, anchor: selected, in: state)
    }

    /// Apply what the panes' change means to the workspace, then keep the layout under the split that came of it.
    func commit(_ panes: TabPanes) {
        guard let anchor = state.selected else { return }
        for action in layouts.actions(for: panes, in: state) { send(action) }
        layouts.save(panes, anchor: anchor, in: state)
    }

    /// Whether the area has room for another pane that way: the core's answer where it has the area's size, else the panes'.
    public func canSplit(_ axis: SplitAxis) -> Bool {
        if let menu = state.layout?.menu { return axis == .row ? menu.canSplitRow : menu.canSplitColumn }
        return layouts.panes(in: state)?.canSplit(axis, room: PaneRoom(0, 0)) ?? false
    }

    /// ⇧⌘↩ and the pane header's zoom button.
    public func toggleZoom() {
        guard var panes = layouts.panes(in: state), panes.toggleZoom() else { return }
        commit(panes)
    }

    /// ⌥⌘← → ↑ ↓: the neighbouring pane takes the keyboard.
    public func focusPane(_ direction: Direction) {
        guard var panes = layouts.panes(in: state), let next = panes.neighbour(direction, room: PaneRoom(1, 1)),
            panes.focus(next)
        else { return }
        commit(panes)
    }
}

extension WindowHub {
    /// What the pane verbs depend on, for the menus: whether the selected terminal's split has several panes,
    /// whether one is zoomed.
    public var paneFacts: (isSplit: Bool, isZoomed: Bool) {
        guard let panes = layouts.panes(in: state) else { return (false, false) }
        return (panes.isSplit, panes.zoomed != nil)
    }
}

extension WindowHub {
    /// One line of how the panes stand, for the control folder's `state`.
    public var paneState: String {
        let panes = layouts.panes(in: state)
        let ids = panes.map { Self.describe($0.tree.root) } ?? ""
        return "panes=\(ids) focused=\(panes.map { String($0.focused) } ?? "none") zoomed=\(panes?.zoomed.map(String.init) ?? "none") "
            + "cover=\(panel.cover.map { "\($0)" } ?? "none")"
    }
}

extension WindowHub {
    /// The tree as text: `1`, `row(1,col(14,15))`.
    private static func describe(_ node: Node) -> String {
        switch node {
        case .leaf(let id): String(id)
        case .split(let axis, let children):
            (axis == .row ? "row(" : "col(") + children.map { describe($0.node) }.joined(separator: ",") + ")"
        }
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
