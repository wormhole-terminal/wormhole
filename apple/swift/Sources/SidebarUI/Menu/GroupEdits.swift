// The one edit of a group the core has no action for yet: its colour (`Groups::set_paint`, `view/sidebar/groups.rs`). The
// window hands it in as a closure when it can do it; without it the menu and the popover leave the colours out. Rename,
// pin, ungroup and the rest are the core's actions, sent as such.

import WorkspaceModel

@MainActor
public struct GroupEdits {
    public var setColor: (_ group: GroupID, _ colour: GroupColor?) -> Void
    public init(setColor: @escaping (GroupID, GroupColor?) -> Void) { self.setColor = setColor }
}
