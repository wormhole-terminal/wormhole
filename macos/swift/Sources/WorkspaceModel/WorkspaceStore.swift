// The one holder of the window's state that the views read: `state` and `send`. It keeps no rule of its own. Every action
// goes to the backend; the state is pulled back, a part at a time, when that part's revision moved; one timer is armed for
// the moment the sidebar's words next change (`refreshIn`) and one wake answers whatever other threads did.

import Observation

@MainActor @Observable
public final class WorkspaceStore {
    public private(set) var state = WorkspaceState()
    /// What the core tells the shell to do that no snapshot carries: the window answers each (`WorkspaceNotice`).
    @ObservationIgnored public var onNotice: @MainActor (WorkspaceNotice) -> Void = { _ in }
    @ObservationIgnored let backend: any WorkspaceBackend
    @ObservationIgnored private var seen: Revisions?
    @ObservationIgnored private var timer: Task<Void, Never>?
    /// The pane area's size, which the core lays the panes out in and refuses a split by.
    @ObservationIgnored private var paneArea = (width: 0.0, height: 0.0)

    public init(backend: any WorkspaceBackend) {
        self.backend = backend
        backend.onWake = { [weak self] in self?.woke() }
        backend.onNotice = { [weak self] in self?.onNotice($0) }
        pull()
    }

    @discardableResult
    public func send(_ action: WorkspaceAction) -> Refusal? {
        let refusal = backend.apply(action)
        pull()
        return refusal
    }

    /// The pane area was laid out at this size (or resized): the core lays the panes out in it.
    public func setPaneArea(width: Double, height: Double) {
        guard (width, height) != paneArea else { return }
        paneArea = (width, height)
        _ = backend.apply(.setPaneArea(width: width, height: height))
        pull(panesMoved: true)
    }

    /// Why dropping a terminal on a pane's edge would be refused; nil when it would work.
    public func paneDropRefusal(dragged: TerminalID, target: TerminalID, zone: Zone) -> Refusal? {
        backend.paneDropRefusal(dragged: dragged, target: target, zone: zone)
    }

    /// Where terminal `id`'s program runs; nil while persistent terminals are off.
    public func hosting(of id: TerminalID) -> HostingState? { backend.hosting(of: id) }

    /// How long ago `since` was (`TerminalSummary.since`), in words.
    public func waited(since: UInt64) -> Waited? { backend.waited(since: since) }
    public func programName(of id: TerminalID) -> String? { backend.programName(of: id) }
    /// What is wrong with terminal `id`'s session host, while something is (the bar over its pane).
    public func hostProblem(of id: TerminalID) -> HostProblem? { backend.hostProblem(of: id) }
    public func reconnect(_ id: TerminalID) { backend.reconnect(id) }
    public func endHost(of id: TerminalID, hard: Bool, done: @escaping @MainActor (String?) -> Void) {
        backend.endHost(of: id, hard: hard, done: done)
    }
    public func startAgain(_ id: TerminalID) { backend.startAgain(id) }

    /// The app is quitting (`WorkspaceBackend.finishQuit`).
    public func finishQuit(endHosts: Bool) { backend.finishQuit(endHosts: endHosts) }
    /// Fill this (empty) window from what it was saved with.
    public func restore() {
        backend.restore()
        pull()
    }

    /// Terminals out of this window for another one, running; nil when none was here.
    public func takeOut(_ ids: [TerminalID]) -> TerminalDeparture? {
        let departure = backend.takeOut(ids)
        pull()
        return departure
    }

    /// What another window gave up, taken in.
    public func receive(_ departure: TerminalDeparture) {
        backend.receive(departure)
        pull()
    }

    /// The backend, for what serves the window beside the store (saving it, the hook socket): never to change it.
    public var source: any WorkspaceBackend { backend }

    /// What dropping a terminal here would do, for the line shown while it is still held.
    public func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan? { backend.dropPlan(dragged: dragged, over: over) }

    /// Where dropping a group here would put it, for the line shown while it is still held.
    public func groupDropPlan(group: GroupID, over: DropOver) -> Landing? { backend.groupDropPlan(group: group, over: over) }

    /// What dragging a column's edge to `extent` would leave, for the readout beside the pointer.
    public func edgeDrag(_ edge: ColumnEdge, extent: Double) -> EdgeDragReadout? { backend.edgeDrag(edge, extent: extent) }

    /// The folder to offer a group that has none before a terminal opens in it.
    public func groupFolderOffer(group: GroupID) -> String? { backend.groupFolderOffer(group: group) }

    /// Whether the group's fence lets the terminal in (a menu greys the group that refuses).
    public func groupAdmits(_ group: GroupID, terminal: TerminalID) -> Bool { backend.groupAdmits(group, terminal: terminal) }

    /// The terminal last selected in a group (nil: the ones in no group).
    public func lastSelected(in group: GroupID?) -> TerminalID? { backend.lastSelected(in: group) }

    /// What VoiceOver hears for each tab of one set, in the bar's order; nil where no core words them.
    public func tabLabels(of ids: [TerminalID]) -> [String]? { backend.tabLabels(of: ids) }

    private func woke() {
        backend.drain()
        pull()
    }

    /// Take the parts whose revision moved (all of them the first time, the sidebar again when its timer fired).
    private func pull(refreshWords: Bool = false, panesMoved: Bool = false) {
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
            (next.search, next.overlay, next.layout) = (chrome.search, chrome.overlay, chrome.layout)
        }
        // The selection decides which split is shown, so the list's moves count too.
        if panesMoved || seen?.chrome != now.chrome || seen?.sidebar != now.sidebar {
            next.panes = backend.panes(width: paneArea.width, height: paneArea.height)
        }
        // A backend that counts no revision of its own for it (the sample) moves it with the list.
        if seen == nil || seen?.terminals != now.terminals || seen?.sidebar != now.sidebar {
            let everything = backend.terminals()
            (next.all, next.counts) = (everything.terminals, everything.counts)
        }
        // The title row follows the selected terminal's facts and the row's own parts: any of the three parts moving.
        if seen == nil || seen?.sidebar != now.sidebar || seen?.chrome != now.chrome || seen?.terminals != now.terminals {
            next.title = backend.title()
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
