// What a pane's header says about its terminal, and what the pane area asks the window to do.

import SwiftUI
import CoreGraphics
import PanesModel
import SidebarUI
import WorkspaceModel

/// A pane's terminal as its header shows it.
public struct PaneInfo: Sendable, Equatable {
    public var title: String
    /// The terminal's folder as the sidebar's second line writes it (`~/Projects/tube`).
    public var folder: String?
    public var activity: Activity
    public var agent: AgentID?

    public init(title: String, folder: String? = nil, activity: Activity = .idle, agent: AgentID? = nil) {
        (self.title, self.folder, self.activity, self.agent) = (title, folder, activity, agent)
    }
}

/// What a pane's buttons, menu, dividers and drops do. Each is a `WorkspaceAction` the window sends: the core lays the panes out
/// and decides what a drop or a divider may do.
public struct PaneActions {
    public var close: (PaneID) -> Void
    public var split: (SplitAxis) -> Void
    public var canSplit: (SplitAxis) -> Bool
    public var leaveSplit: (PaneID) -> Void
    public var focus: (PaneID) -> Void
    public var toggleZoom: () -> Void
    /// A divider dragged to a position along its axis; equalize on a double-click.
    public var dragDivider: (PaneDividerState.ID, Double) -> Void
    public var equalize: (PaneDividerState.ID) -> Void
    /// A terminal dropped on a pane's edge; `refusal` says beforehand whether the core would refuse it.
    public var drop: (DraggedTerminal, PaneID, Zone) -> Void
    public var refusal: (DraggedTerminal, PaneID, Zone) -> Refusal?
    /// The area was laid out at this size.
    public var areaChanged: (CGSize) -> Void
    /// A pane's tile as the terminal's row in the list draws it (the agent's logo, the status light); nil draws the plain tile.
    public var tile: ((PaneID, CGFloat) -> TerminalStatusTile?)?
    /// The key the keymap has now for an action (`splitRight`, `splitDown`, `zoomPane`, `closeTerminal`), as a menu item binds it and
    /// as a tooltip says it ("⇧⌘↩"); nil when it has none. Without the window's keymap the menu and tooltips show no key.
    public var shortcut: (String) -> KeyboardShortcut? = { _ in nil }
    public var keyText: (String) -> String? = { _ in nil }
    /// The core's rule for a split's opening motion; nil (the sample) draws a split as it lands.
    public var splitMotion: (any SplitMotion)?
    /// The terminal the window's last split opened, if any: only its coming plays the opening (a drop or a join is a cut).
    public var splitCame: () -> PaneID? = { nil }

    public init(
        close: @escaping (PaneID) -> Void = { _ in }, split: @escaping (SplitAxis) -> Void = { _ in },
        canSplit: @escaping (SplitAxis) -> Bool = { _ in true }, leaveSplit: @escaping (PaneID) -> Void = { _ in },
        focus: @escaping (PaneID) -> Void = { _ in }, toggleZoom: @escaping () -> Void = {},
        dragDivider: @escaping (PaneDividerState.ID, Double) -> Void = { _, _ in },
        equalize: @escaping (PaneDividerState.ID) -> Void = { _ in },
        drop: @escaping (DraggedTerminal, PaneID, Zone) -> Void = { _, _, _ in },
        refusal: @escaping (DraggedTerminal, PaneID, Zone) -> Refusal? = { _, _, _ in nil },
        areaChanged: @escaping (CGSize) -> Void = { _ in }
    ) {
        (self.close, self.split, self.canSplit, self.leaveSplit, self.focus) = (close, split, canSplit, leaveSplit, focus)
        (self.toggleZoom, self.dragDivider, self.equalize, self.drop, self.refusal, self.areaChanged) =
            (toggleZoom, dragDivider, equalize, drop, refusal, areaChanged)
    }
}
