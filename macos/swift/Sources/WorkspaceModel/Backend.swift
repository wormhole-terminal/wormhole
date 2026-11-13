// The seam between the window's state and whatever keeps it: the Rust workspace through the FFI (`WorkspaceBridge`) or
// the in-memory sample (`MockWorkspaceBackend`). A backend answers in this module's values; the store pulls a snapshot
// only when its revision moved, so a window with nothing happening does nothing.

import Foundation

/// What a backend counts: a part's snapshot changed when its number did.
public struct Revisions: Sendable, Equatable {
    public var sidebar: UInt64
    public var chrome: UInt64
    /// The all-terminals snapshot (`terminals()`); the sample never moves it.
    public var terminals: UInt64

    public init(sidebar: UInt64, chrome: UInt64, terminals: UInt64 = 0) {
        self.sidebar = sidebar
        self.chrome = chrome
        self.terminals = terminals
    }
}

/// The window's own switches (core `ChromeSnapshot`).
public struct ChromeState: Sendable, Equatable {
    public var sidebarVisible: Bool
    public var panelVisible: Bool
    public var panelTab: PanelTab
    public var search: String
    public var overlay: Overlay?
    /// The columns, tabs and layouts as the core fitted them; nil without a window width.
    public var layout: ChromeLayout?

    public init(
        sidebarVisible: Bool = true, panelVisible: Bool = true, panelTab: PanelTab = .changes, search: String = "",
        overlay: Overlay? = nil, layout: ChromeLayout? = nil
    ) {
        self.layout = layout
        self.sidebarVisible = sidebarVisible
        self.panelVisible = panelVisible
        self.panelTab = panelTab
        self.search = search
        self.overlay = overlay
    }
}

/// Why an action changed nothing (core `Refusal`): data, not an error.
public enum Refusal: String, Sendable, Equatable {
    case unknown, tooSmall, unchanged
}

@MainActor
public protocol WorkspaceBackend: AnyObject {
    /// Set by the store: called on the main actor, once, when something changed away from a call of the store's
    /// (a terminal's title, folder or bell). The store answers with `drain()`.
    var onWake: (@MainActor () -> Void)? { get set }
    /// Every terminal that exists, in the order they were made.
    var live: [TerminalID] { get }
    /// Do what the person did; why it changed nothing, if it did not.
    func apply(_ action: WorkspaceAction) -> Refusal?
    /// Take what other threads caused since the last drain; this re-arms the wake.
    func drain()
    func revisions() -> Revisions
    func sidebar() -> SidebarSnapshot
    func chrome() -> ChromeState
    /// The selected terminal's panes in an area of this size.
    func panes(width: Double, height: Double) -> PanesState
    /// Why dropping `dragged` on `target`'s `zone` would be refused (a pane of the split moves, any other terminal joins); nil
    /// when it would work. Pure.
    func paneDropRefusal(dragged: TerminalID, target: TerminalID, zone: Zone) -> Refusal?
    /// What dropping `dragged` on `over` would do; nil when nothing would change. Pure: nothing moves.
    func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan?
    /// Where a dragged group would land on `over`; nil when nothing would change. Pure.
    func groupDropPlan(group: GroupID, over: DropOver) -> Landing?
    /// Every terminal, folded ones too, with the status counts.
    func terminals() -> TerminalsSnapshot
    /// The selected terminal's title row.
    func title() -> TitleState?
    /// What a drag of a column's edge would leave, for its readout.
    func edgeDrag(_ edge: ColumnEdge, extent: Double) -> EdgeDragReadout?
    /// The folder to ask about before a terminal opens in a group that has none, when one is worth offering.
    func groupFolderOffer(group: GroupID) -> String?
    /// Whether the group's fence lets the terminal (and its split) in: a group that refuses is greyed in the menus.
    func groupAdmits(_ group: GroupID, terminal: TerminalID) -> Bool
    /// The terminal last selected in a group (nil: the terminals in no group): what opening a tab set selects.
    func lastSelected(in group: GroupID?) -> TerminalID?
    /// What VoiceOver hears for each of these terminals as the tabs of one set, in order (the core's words: name, state,
    /// selection, "n of m"); nil in the sample, whose tabs say their name.
    func tabLabels(of ids: [TerminalID]) -> [String]?
    /// The window came to the front or went behind (`active`) and whether it can be seen at all (`seen`): in front, what is
    /// on screen is looked at, so a finished result is spent and a bell heard.
    func setWindowState(active: Bool, seen: Bool)
    /// Where terminal `id`'s program runs; nil while persistent terminals are off (the status bar then says nothing).
    func hosting(of id: TerminalID) -> HostingState?
    /// How long ago a terminal's `since` was, as the status bar says it; nil in the sample (no clock of the core's).
    func waited(since: UInt64) -> Waited?
    /// What runs in terminal `id`, whatever title it set: the agent, the running command or the shell (nil in the sample).
    func programName(of id: TerminalID) -> String?
    /// What is wrong with terminal `id`'s host, while something is; nil for a terminal that is fine and for one in the app.
    func hostProblem(of id: TerminalID) -> HostProblem?
    /// "Try again": attach to the host a terminal could not be shown from. The pane is blank until it is done.
    func reconnect(_ id: TerminalID)
    /// "End host": ask the host to end (`hard` for one that does not answer); `done` says why it did not work, nil when it
    /// did, in which case the backend tells the window to close the terminal (`WorkspaceNotice.hostEnded`).
    func endHost(of id: TerminalID, hard: Bool, done: @escaping @MainActor (String?) -> Void)
    /// "Open a new terminal here" and Restart: a new program in this terminal's place, in a new host when they are on.
    func startAgain(_ id: TerminalID)
    /// The app is quitting: `endHosts` ends every terminal's host (Quit and End All Terminals), otherwise each is let go
    /// of and its program runs on.
    func finishQuit(endHosts: Bool)
    /// The settings changed: what the core keeps of the terminals follows them (saved text erased when its setting went
    /// off, checkpoints stopped).
    func settingsChanged()
    /// Fill an empty window from what it was saved with: its terminals come back (programs started afresh in their
    /// folders), with their names, pins, groups, splits, columns and selection. Nothing for a window with no record.
    func restore()
    /// Take terminals out of this window for another one, their programs running; nil when none of them is here (or the
    /// backend cannot hand a terminal over). Here they are as good as closed.
    func takeOut(_ ids: [TerminalID]) -> TerminalDeparture?
    /// Take in what another window gave up. A departure that this kind of backend did not make is ignored.
    func receive(_ departure: TerminalDeparture)
    /// Set by the store: told what no snapshot carries (a screen's saved text to forget, a group to move to a window).
    var onNotice: (@MainActor (WorkspaceNotice) -> Void)? { get set }
}
