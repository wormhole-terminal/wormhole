// The edits of a group the window's model has no action for yet: its name, its colour, pinning, ungrouping and closing
// (`Groups::rename`, `set_paint`, `pin`, `ungroup` in `view/sidebar/groups.rs`). The window hands them in as closures when it
// can do them; without them the group's menu and popover offer only fold, open and a new terminal. When the model gains
// `WorkspaceAction` cases for these, they become one-line forwards and this seam goes.

import WorkspaceModel

@MainActor
public struct GroupEdits {
    public var rename: (_ group: String, _ name: String) -> Void
    public var setColor: (_ group: String, _ colour: FolderColor?) -> Void
    public var togglePin: (_ group: String) -> Void
    public var ungroup: (_ group: String) -> Void
    public var close: (_ group: String) -> Void
    public init(
        rename: @escaping (String, String) -> Void, setColor: @escaping (String, FolderColor?) -> Void,
        togglePin: @escaping (String) -> Void, ungroup: @escaping (String) -> Void, close: @escaping (String) -> Void
    ) {
        (self.rename, self.setColor, self.togglePin, self.ungroup, self.close) = (rename, setColor, togglePin, ungroup, close)
    }
}
