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
}
