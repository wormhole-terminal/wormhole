// What one window holds beyond the workspace's state, and the one place its parts reach each other: the
// notice chip, the title row's arrangement, the panel and each terminal's overlay. The views
// read it (it is observable) and call its methods; the app gives it the store, the terminals and, through
// `HubActions`, what only the app can do (ask a question before closing, start a terminal again).

import AppKit
import Observation
import PaletteModel
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
    public let sidebarLink = SidebarLink()
    /// Where the keyboard is among the window's areas, and where F6, ⇥ and Esc take it.
    @ObservationIgnored public private(set) lazy var focus = WindowFocus(hub: self)
    /// A terminal dragged out of the sidebar over the panes: the list reports it, the pane area takes the drop.
    public let terminalDrag = TerminalDragPointer()
    /// Each terminal's agent's model and context window, as its status line last said (the title row's gauge chip).
    public let gauges = GaugeBoard()
    public var actions = HubActions()
    /// The terminal welcome's example band in the sidebar, while the program's sidebar step is up (never a terminal).
    public var exampleBand: ExampleBand?
    /// The sidebar's note after a first run without agents (`FirstRun::agents_note`).
    public var agentsTip = AgentsTip.off
    /// A click on the chip that says an update is ready: the app restarts into it (asking first when that ends something).
    public var onRestartToUpdate: (() -> Void)?
    /// The Project tab's "View all N…": the app opens the window of the project's commands.
    public var onShowProjectCommands: (() -> Void)?
    /// Whether Open in and Reveal start other apps: a test instance only says it did (as the GPUI app's does), so a check
    /// never opens the person's editor or Finder.
    public var launchesApps = true
    /// The Fold output feature is on (the context menu offers Fold Output).
    public var foldsOn = false
    /// Esc ends a column drag in progress (the window body sets it): whether there was one.
    @ObservationIgnored var cancelColumnDrag: (() -> Bool)?
    /// The title row's parts as arranged (View ▸ Title Row, the row's own menu).
    public var titleRow = TitleRow.default
    /// Zen mode is on: the row is the preset's, and arranging it would change the person's own (`settings::zen`).
    public var zenActive = false
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
    @ObservationIgnored var overlays: [TerminalID: TerminalOverlay] = [:]

    public init(store: WorkspaceStore, terminals: TerminalViews, panelBackends: PanelBackends = .mock()) {
        self.store = store
        self.terminals = terminals
        let host = SystemPanelHost()
        panel = PanelController(host: host, backends: panelBackends)
        panel.presentation = .terminalArea
        connect(host)
        notices.onAct = { [unowned self] in noticeActed($0) }
        notices.onBar = { [unowned self] in sayInBar($0, detail: $1) }
        followGauges()
    }

    /// A gauge goes once the shell has its terminal back, so the agent's next run starts without the last one's numbers.
    private func followGauges() {
        let all = withObservationTracking { store.state.all } onChange: { [weak self] in
            Task { @MainActor in self?.followGauges() }
        }
        gauges.forgetGone(all)
    }

    public var state: WorkspaceState { store.state }
    /// The tab Usage was opened from: Usage returns to it when its agent leaves (`returns_to_previous`).
    @ObservationIgnored var tabBeforeUsage: WorkspaceModel.PanelTab?

    public func send(_ action: WorkspaceAction) {
        // Before the overlay's field takes the keyboard, and after it is gone: the keyboard goes back where it was.
        if case .showOverlay(let next) = action {
            focus.overlayChanging(to: next != nil)
            defer { if next == nil { focus.overlayClosed() } }
            _ = store.send(action)
            return
        }
        guard case .showPanelTab(let tab) = action else {
            _ = store.send(action)
            return
        }
        // A tab that is not on offer (Usage without an agent) is gone, as in the GPUI app's `show_tab`.
        let shown = panelTabs.contains(tab) ? tab : .changes
        if shown == .usage, state.panelTab != .usage { tabBeforeUsage = state.panelTab }
        _ = store.send(.showPanelTab(shown))
    }
}

/// What the window asks of the app that the views cannot do themselves.
public struct HubActions {
    /// Closing a terminal asks first when something runs in it.
    public var closeTerminal: @MainActor (TerminalID) -> Void = { _ in }
    /// Closing a group asks first when something works in it.
    public var closeGroup: @MainActor (GroupID) -> Void = { _ in }
    /// "Don't show again" on the sidebar's note: kept for good, until Settings' Reset tips.
    public var dismissAgentsTip: @MainActor () -> Void = {}
    /// Settings at a page ("agents"), from the note's link.
    public var openSettingsAt: @MainActor (String) -> Void = { _ in }
    /// The shells a right-click of a + offers, and what choosing one does (the menu bar's New Terminal With ▸).
    public var shells: @MainActor () -> [ShellOffer] = { [] }
    public var newTerminalWith: @MainActor (String) -> Void = { _ in }
    /// The Show terminals in ▸ Sidebar choice (the tab bar's group switcher offers it).
    public var showTerminalsInSidebar: @MainActor () -> Void = {}
    /// A group's native menu, for a right-click on its head or on its card in the tab bar's switcher.
    public var groupMenu: @MainActor (GroupID) -> NSMenu? = { _ in nil }
    /// Settings, open on the setting with this id (`RevealSetting`).
    public var revealSetting: @MainActor (String) -> Void = { _ in }
    /// Whether this window's profile is the one Settings shows: a window of another profile cannot arrange its title row.
    public var editsSettings: @MainActor () -> Bool = { true }
    /// A menu the keys opened (⌃↩), under `frame` (in the window's content, from its top left): shown, or kept in a test instance.
    public var showKeyMenu: @MainActor (NSMenu, CGRect) -> Void = { _, _ in }
    /// The native context menu of a terminal's grid, built when it is asked for.
    public var terminalMenu: @MainActor (TerminalID) -> NSMenu? = { _ in nil }
    /// A keymap action by id (the palette's): the app's router runs it.
    public var perform: @MainActor (String) -> Void = { _ in }
    /// What the palette and the shortcut sheet list and do (the core's actions, settings and commits).
    public var palette = PaletteSources()
    /// Settings, where the title row's parts are also arranged.
    public var openSettings: @MainActor () -> Void = {}
    /// The profiles the tab bar's chip offers (none while there is one) and what its menu does.
    public var profiles: @MainActor () -> [ProfileChoice] = { [] }
    public var profile = ProfileActions()
    /// Every terminal of every window in sight, for the status bar's counts (nil: this window's own), and what a click on one does:
    /// select it and bring its window forward.
    public var everyTerminal: (@MainActor () -> [TerminalSummary])?
    public var showTerminal: (@MainActor (TerminalID) -> Void)?
    /// The profile this window is in when it is not the one Wormhole is in (nil otherwise): the status bar names it.
    public var windowProfile: (@MainActor () -> ProfileChoice?)?
    /// The branch picker (needs the repository; the core's `git` answers it).
    public var pickBranch: @MainActor (TerminalID) -> Void = { _ in }

    public init() {}
}
