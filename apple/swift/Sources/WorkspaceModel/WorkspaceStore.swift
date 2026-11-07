// The one holder of the window's state that the views read: `state` and `send`. It keeps no rule of its own. Every action
// goes to the backend; the state is pulled back, a part at a time, when that part's revision moved; one timer is armed for
// the moment the sidebar's words next change (`refreshIn`) and one wake answers whatever other threads did.

import Observation

@MainActor @Observable
public final class WorkspaceStore {
    public private(set) var state = WorkspaceState()
    @ObservationIgnored private let backend: any WorkspaceBackend
    @ObservationIgnored private var seen: Revisions?
    @ObservationIgnored private var timer: Task<Void, Never>?

    public init(backend: any WorkspaceBackend) {
        self.backend = backend
        backend.onWake = { [weak self] in self?.woke() }
        pull()
    }

    @discardableResult
    public func send(_ action: WorkspaceAction) -> Refusal? {
        if case .newTerminalInGroup(let group) = action {
            // The new terminal is the selected one; the core has no "new terminal in group" of its own.
            guard send(.newTerminal) == nil, let id = state.selected else { return .unchanged }
            return send(.addToGroup(id, group))
        }
        let refusal = backend.apply(action)
        pull()
        return refusal
    }

    /// What dropping a terminal here would do, for the line shown while it is still held.
    public func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan? { backend.dropPlan(dragged: dragged, over: over) }

    private func woke() {
        backend.drain()
        pull()
    }

    /// Take the parts whose revision moved (all of them the first time, the sidebar again when its timer fired).
    private func pull(refreshWords: Bool = false) {
        let now = backend.revisions()
        var next = state
        if refreshWords || seen?.sidebar != now.sidebar {
            let list = backend.sidebar()
            next.sections = list.sections
            next.selected = list.selected
            arm(after: list.refreshIn)
        }
        if seen?.chrome != now.chrome {
            let chrome = backend.chrome()
            (next.sidebarVisible, next.panelVisible, next.panelTab) = (chrome.sidebarVisible, chrome.panelVisible, chrome.panelTab)
            (next.search, next.overlay) = (chrome.search, chrome.overlay)
        }
        next.live = backend.live
        seen = now
        if next != state { state = next }
    }

    /// One timer, never a poll: the words that move with time ("Working · 3m") are read again when they change.
    private func arm(after wait: Duration?) {
        timer?.cancel()
        guard let wait else { return timer = nil }
        timer = Task { [weak self] in
            // A hair past the instant, so the words are read after they change.
            try? await Task.sleep(for: wait + .milliseconds(10))
            guard !Task.isCancelled else { return }
            self?.pull(refreshWords: true)
        }
    }
}
