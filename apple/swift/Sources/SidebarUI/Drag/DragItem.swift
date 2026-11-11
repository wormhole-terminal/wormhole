// What is being dragged in the list: a terminal by its row, or a group by its head.

import WorkspaceModel

enum DragItem: Hashable, Sendable {
    case terminal(TerminalID)
    case group(GroupID)
}

/// Where the pointer can land, measured in the list's own coordinates (`DropSlot` frames).
enum DropSlot: Hashable, Sendable {
    case row(TerminalID)
    case head(GroupID)
    /// A group's whole card, for the outline that says a drop joins it.
    case card(GroupID)
}
