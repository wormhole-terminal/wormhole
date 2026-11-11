// The window's state kept by the Rust workspace (`wormhole-ffi` `Workspace`, core's `workspace::WorkspaceModel`). Swift
// sends actions and pulls snapshots when a revision moved; it decides nothing. What it does itself is what the core cannot:
// start a program when a terminal is opened (and hand it to the workspace, which feeds the row from it), forget a view when
// one is closed (Rust has already ended the program) and hop a Rust thread's wake to the main actor, once.

import Foundation
import TerminalCore
import UniFFISource
import WorkspaceModel
import WormholeBindings

/// What a window starts its terminals with, and what the core is told of the machine.
public struct WorkspaceSetup: Sendable {
    public var program: String
    public var arguments: [String]
    public var home: String?
    /// ⌘1…⌘9 count along the list as shown (`true`), or stay with their terminal.
    public var numbersFollowList: Bool
    /// Terminals in session hosts that outlive the app; nil keeps them all in the app's process.
    public var hosting: HostingSetup?
    /// What a pane's header adds to the least height of a pane: the taller of the gapped header (tile 18 + 2 x 8) and the
    /// row's 28, as the GPUI app's `style::pane_header_h(true).max(PANE_HEADER_H)` says.
    public var paneHeaderHeight: Double = 34

    public init(
        program: String, arguments: [String] = [], home: String? = NSHomeDirectory(), numbersFollowList: Bool = false,
        hosting: HostingSetup? = nil
    ) {
        (self.program, self.arguments, self.home, self.numbersFollowList) = (program, arguments, home, numbersFollowList)
        self.hosting = hosting
    }
}

@MainActor
public final class RustWorkspaceBackend: WorkspaceBackend, AgentHookSink {
    public var onWake: (@MainActor () -> Void)?
    public var onNotice: (@MainActor (WorkspaceNotice) -> Void)?
    public private(set) var live: [TerminalID] = []

    /// The core's workspace, for what serves it to others (the hook socket).
    public let core: Workspace
    let setup: WorkspaceSetup
    var relays: [TerminalID: TerminalRelay] = [:]
    var failed: [TerminalID: String] = [:]
    /// The sources made for terminals, to let go of one that moves to another window (the view's `stop` would end it).
    var sources: [TerminalID: WeakSource] = [:]
    /// The persistent terminals' object, nil while the feature is off or unusable (`RustWorkspaceBackend+Hosting`).
    let hosted: HostedSessions?
    /// Persistent terminals are wanted but the app runs from a disk image or a temporary copy: the person is told.
    let needsApplications: Bool
    var book = HostBook()
    /// What the terminals of a window restored from its record were, by their ids (they keep them): each attaches to its
    /// own host, or starts again where its host is gone.
    var saved: [TerminalID: HostedSessions.Entry] = [:]
    /// What the next terminals opened are, in order: hosts nobody named, adopted.
    var adopting: [HostedSessions.Entry] = []
    /// Why terminals stay in the app though persistence is on, once a host did not start.
    public internal(set) var hostingFallback: String?

    /// The size a terminal starts at, until its view fits it: the view's first layout resizes it.
    static let startSize = GridSize(cols: 100, rows: 30)

    /// A workspace of its own, or window `window` of `windows`: its terminal ids are the app's, never another window's.
    public init(setup: WorkspaceSetup, windows: Windows? = nil, window: Int = 0) {
        self.setup = setup
        switch setup.hosting.map(HostedSessions.open) {
        case .on(let sessions)?: (hosted, needsApplications) = (sessions, false)
        case .needsApplications?: (hosted, needsApplications) = (nil, true)
        case .off?, nil: (hosted, needsApplications) = (nil, false)
        }
        let wake = WakeRelay()
        let config = WorkspaceConfig(
            home: setup.home, numbersFollowList: setup.numbersFollowList, startFolder: setup.home,
            paneHeaderHeight: Float(setup.paneHeaderHeight))
        core = windows.map { Workspace.inWindow(windows: $0, window: Int64(window), config: config, listener: wake) }
            ?? Workspace(config: config, listener: wake)
        wake.attach { [weak self] in self?.onWake?() }
    }

    /// The frame source of terminal `id`, over the program started for it; nil when it did not start.
    public func frameSource(for id: TerminalID) -> (any FrameSource)? {
        guard let relay = relays[id] else { return nil }
        // A terminal still being started in or attached to a host has no program yet: its view waits and draws it when it comes.
        let source = core.terminal(id: Int64(id)).map { UniFFIFrameSource(adopting: $0, relay: relay) }
            ?? UniFFIFrameSource(waiting: relay)
        source.onHostResponsiveness { [weak self] in Task { @MainActor in self?.onNotice?(.hostChanged(id)) } }
        sources[id] = WeakSource(source)
        return source
    }

    /// Why a terminal's program did not start, for the window to say.
    public func startFailure(of id: TerminalID) -> String? { failed[id] }

    /// The folder the next terminal opened starts in, when the action named one (`newTerminalIn`): the core opens a terminal where
    /// the selected one is, and the folder is the program's to start in.
    private var folderOfNext: String?

    public func apply(_ action: WorkspaceAction) -> WorkspaceModel.Refusal? {
        defer {
            folderOfNext = nil
            syncVisibility()
        }
        if case .newTerminalIn(let folder) = action { folderOfNext = folder }
        return handle(core.apply(action: WormholeBindings.Action(action)))
    }

    public func drain() {
        _ = handle(core.drain())
        syncVisibility()
    }

    public func setWindowState(active: Bool, seen: Bool) { _ = handle(core.setWindowState(active: active, seen: seen)) }

    public func hook(id: TerminalID, agent: String, event: String, message: String?) {
        _ = handle(core.hook(id: Int64(id), agent: agent, event: event, message: message))
        // A hook fed from the main thread wakes no one: the window pulls what changed itself.
        onWake?()
    }

    public func setNotes(id: TerminalID, tasksDone: [Bool], notes: [String]) {
        _ = handle(core.setNotes(id: Int64(id), tasksDone: tasksDone, notes: notes))
        onWake?()
    }

    public func revisions() -> WorkspaceModel.Revisions { WorkspaceModel.Revisions(core.revisions()) }
    public func sidebar() -> WorkspaceModel.SidebarSnapshot { WorkspaceModel.SidebarSnapshot(core.sidebar()) }
    public func chrome() -> ChromeState { ChromeState(core.chrome()) }
    public func panes(width: Double, height: Double) -> PanesState { PanesState(core.panes(width: Float(width), height: Float(height))) }

    public func paneDropRefusal(dragged: TerminalID, target: TerminalID, zone: WorkspaceModel.Zone) -> WorkspaceModel.Refusal? {
        core.paneDropRefusal(dragged: Int64(dragged), target: Int64(target), zone: WormholeBindings.Zone(zone)).map(WorkspaceModel.Refusal.init)
    }
    public func terminals() -> WorkspaceModel.TerminalsSnapshot { WorkspaceModel.TerminalsSnapshot(core.terminals()) }
    public func title() -> TitleState? { TitleState(core.title(bootVolume: Self.bootVolume)) }

    /// What the system calls the startup volume ("Macintosh HD"), which the title row's path menu names the root with.
    private static let bootVolume =
        (try? URL(fileURLWithPath: "/").resourceValues(forKeys: [.volumeNameKey]).volumeName) ?? "Macintosh HD"

    public func dropPlan(dragged: TerminalID, over: DropOver) -> WorkspaceModel.DropPlan? {
        core.dropPlan(dragged: Int64(dragged), over: Over(over)).map(WorkspaceModel.DropPlan.init)
    }

    public func groupDropPlan(group: GroupID, over: DropOver) -> WorkspaceModel.Landing? {
        core.groupDropPlan(group: group, over: Over(over)).map(WorkspaceModel.Landing.init)
    }

    public func edgeDrag(_ edge: ColumnEdge, extent: Double) -> EdgeDragReadout? {
        EdgeDragReadout(core.edgeDrag(edge: WormholeBindings.Edge(edge), extent: Float(extent)))
    }

    public func groupFolderOffer(group: GroupID) -> String? { core.groupFolderOffer(group: group) }

    /// What the core says happened: start the program of a terminal opened, forget one closed; a refusal is the answer.
    func handle(_ events: [WorkspaceEvent]) -> WorkspaceModel.Refusal? {
        var refusal: WorkspaceModel.Refusal?
        for event in events {
            switch event {
            case .opened(let id, let cwd):
                live.append(Int(id))
                start(Int(id), in: folderOfNext ?? cwd)
            case .closed(let id):
                live.removeAll { $0 == Int(id) }
                book.forget(Int(id))
                relays[Int(id)] = nil
                sources[Int(id)] = nil
                failed[Int(id)] = nil
            case .adopted(let id): live.append(Int(id))
            case .refused(let reason): refusal = WorkspaceModel.Refusal(reason)
            case .savedTextForgotten(let id): onNotice?(.savedTextForgotten(Int(id)))
            case .groupToWindow(let group, let terminals): onNotice?(.groupToWindow(group, terminals: terminals.map(Int.init)))
            case .selected, .moved, .listing, .layout, .chrome: break
            }
        }
        return refusal
    }
}
