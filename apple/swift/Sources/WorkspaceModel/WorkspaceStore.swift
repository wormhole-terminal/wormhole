// The one mutable holder of the window's state: the UI reads `state` and calls `send`, never edits it.

import Observation

@MainActor @Observable
public final class WorkspaceStore {
    public private(set) var state: WorkspaceState

    public init(state: WorkspaceState = WorkspaceState()) {
        self.state = state
    }

    public func send(_ action: WorkspaceAction) {
        state = reduce(state, action)
    }

    /// A terminal's own title (OSC 0/2) replaces its row's; an empty one keeps what the row has.
    public func setTitle(_ title: String, of id: TerminalID) {
        guard !title.isEmpty, let index = state.terminals.firstIndex(where: { $0.id == id }) else { return }
        state.terminals[index].title = title
    }
}
