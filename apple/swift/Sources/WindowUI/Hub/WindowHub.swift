// What one window holds beyond the workspace's state, and the one place its parts reach each other: the pane
// layouts, the notice chip, the title row's arrangement, the panel and each terminal's overlay. The views
// read it (it is observable) and call its methods; the app gives it the store, the terminals and, through
// `HubActions`, what only the app can do (ask a question before closing, start a terminal again).

import AppKit
import Observation
import PanelModel
import PanelUI
import PanesModel
import PanesUI
import SidebarUI
import TerminalUI
import WorkspaceModel

@MainActor @Observable
public final class WindowHub {
    public let store: WorkspaceStore
    public let terminals: TerminalViews
    public let notices = NoticeCenter()
    public let drag = PaneDragSession()
    /// ⌘F while the sidebar has the keyboard is its search's, else the terminal's find.
    public let sidebarSearch = SidebarSearch()
    /// A terminal dragged out of the sidebar over the panes: the list reports it, the pane area takes the drop.
    public let terminalDrag = TerminalDragPointer()
    /// Each terminal's agent's model and context window, as its status line last said (the title row's gauge chip).
    public let gauges = GaugeBoard()
    public var actions = HubActions()
    /// The title row's parts as arranged (View ▸ Title Row, the row's own menu).
    public var titleRow = TitleRow.default
    /// The title row's order of giving way: the core's where the FFI is linked, else every part at its natural width.
    public var rowFitting: any RowFitting = NaturalFit()
    /// The right panel's state over the backends the app hands in (the core's git where the FFI is linked, else the
    /// sample). An opened diff or file is laid over the terminals (`.terminalArea`, hosted by `TerminalColumn`).
    public let panel: PanelController
    public var openIn: any OpenInBackend = SystemOpenIn()
    /// The program's name for the exited bar ("zsh exited").
    public var shell = "zsh"
    /// Bumped to make a terminal's host start over with a fresh view (a shell that ended starts again).
    var restarts: [TerminalID: Int] = [:]
    var layouts = PaneLayouts()
    @ObservationIgnored var overlays: [TerminalID: TerminalOverlay] = [:]

    public init(store: WorkspaceStore, terminals: TerminalViews, panelBackends: PanelBackends = .mock()) {
        self.store = store
        self.terminals = terminals
        let host = SystemPanelHost()
        panel = PanelController(host: host, backends: panelBackends)
        panel.presentation = .terminalArea
        connect(host)
    }

    public var state: WorkspaceState { store.state }
    public func send(_ action: WorkspaceAction) { store.send(action) }
}

/// What the window asks of the app that the views cannot do themselves.
public struct HubActions {
    /// Closing a terminal asks first when something runs in it.
    public var closeTerminal: @MainActor (TerminalID) -> Void = { _ in }
    /// Closing a group asks first when something works in it.
    public var closeGroup: @MainActor (GroupID) -> Void = { _ in }
    /// The Show terminals in ▸ Sidebar choice (the tab bar's group switcher offers it).
    public var showTerminalsInSidebar: @MainActor () -> Void = {}
    /// The native context menu of a terminal's grid, built when it is asked for.
    public var terminalMenu: @MainActor (TerminalID) -> NSMenu? = { _ in nil }
    /// A keymap action by id (the palette's): the app's router runs it.
    public var perform: @MainActor (String) -> Void = { _ in }
    /// Settings, where the title row's parts are also arranged.
    public var openSettings: @MainActor () -> Void = {}
    /// The branch picker (needs the repository; the core's `git` answers it).
    public var pickBranch: @MainActor (TerminalID) -> Void = { _ in }

    public init() {}
}
