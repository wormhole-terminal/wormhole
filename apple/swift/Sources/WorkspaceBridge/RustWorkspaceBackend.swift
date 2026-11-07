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

    public init(program: String, arguments: [String] = [], home: String? = NSHomeDirectory(), numbersFollowList: Bool = false) {
        (self.program, self.arguments, self.home, self.numbersFollowList) = (program, arguments, home, numbersFollowList)
    }
}

@MainActor
public final class RustWorkspaceBackend: WorkspaceBackend, AgentHookSink {
    public var onWake: (@MainActor () -> Void)?
    public var onNotice: (@MainActor (WorkspaceNotice) -> Void)?
    public private(set) var live: [TerminalID] = []

    /// The core's workspace, for what serves it to others (the hook socket).
    public let core: Workspace
    private let setup: WorkspaceSetup
    private var relays: [TerminalID: TerminalRelay] = [:]
    private var failed: [TerminalID: String] = [:]

    /// The size a terminal starts at, until its view fits it: the view's first layout resizes it.
    private static let startSize = GridSize(cols: 100, rows: 30)

    public init(setup: WorkspaceSetup) {
        self.setup = setup
        let wake = WakeRelay()
        core = Workspace(
            config: WorkspaceConfig(
                home: setup.home, numbersFollowList: setup.numbersFollowList, startFolder: setup.home, paneHeaderHeight: 0),
            listener: wake)
        wake.attach { [weak self] in self?.onWake?() }
    }

    /// The frame source of terminal `id`, over the program started for it; nil when it did not start.
    public func frameSource(for id: TerminalID) -> (any FrameSource)? {
        guard let relay = relays[id], let terminal = core.terminal(id: Int64(id)) else { return nil }
        return UniFFIFrameSource(adopting: terminal, relay: relay)
    }

    /// Why a terminal's program did not start, for the window to say.
    public func startFailure(of id: TerminalID) -> String? { failed[id] }

    public func apply(_ action: WorkspaceAction) -> WorkspaceModel.Refusal? {
        handle(core.apply(action: WormholeBindings.Action(action)))
    }

    public func drain() { _ = handle(core.drain()) }

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
    private func handle(_ events: [WorkspaceEvent]) -> WorkspaceModel.Refusal? {
        var refusal: WorkspaceModel.Refusal?
        for event in events {
            switch event {
            case .opened(let id, let cwd): open(Int(id), in: cwd)
            case .closed(let id):
                live.removeAll { $0 == Int(id) }
                relays[Int(id)] = nil
                failed[Int(id)] = nil
            case .refused(let reason): refusal = WorkspaceModel.Refusal(reason)
            case .savedTextForgotten(let id): onNotice?(.savedTextForgotten(Int(id)))
            case .groupToWindow(let group, let terminals): onNotice?(.groupToWindow(group, terminals: terminals.map(Int.init)))
            case .selected, .moved, .listing, .layout, .chrome: break
            }
        }
        return refusal
    }

    private func open(_ id: TerminalID, in folder: String?) {
        live.append(id)
        let relay = TerminalRelay()
        do {
            let size = Self.startSize
            // Started for its tab: its shell has `WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and `WORMHOLE_BIN`, so agents' hooks
            // and its own `wh` reach this workspace.
            let terminal = try Terminal.startForTab(
                tab: Int64(id), program: setup.program, args: setup.arguments, cwd: folder, cols: UInt16(size.cols),
                rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight, listener: relay)
            relays[id] = relay
            core.registerTerminal(id: Int64(id), terminal: terminal)
        } catch {
            failed[id] = "\(error)"
            FileHandle.standardError.write(Data("terminal \(id): cannot start \(setup.program): \(error)\n".utf8))
        }
    }
}
