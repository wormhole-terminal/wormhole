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
public final class RustWorkspaceBackend: WorkspaceBackend {
    public var onWake: (@MainActor () -> Void)?
    public private(set) var live: [TerminalID] = []

    private let workspace: Workspace
    private let setup: WorkspaceSetup
    private var relays: [TerminalID: TerminalRelay] = [:]
    private var failed: [TerminalID: String] = [:]

    /// The size a terminal starts at, until its view fits it: the view's first layout resizes it.
    private static let startSize = GridSize(cols: 100, rows: 30)

    public init(setup: WorkspaceSetup) {
        self.setup = setup
        let wake = WakeRelay()
        workspace = Workspace(
            config: WorkspaceConfig(
                home: setup.home, numbersFollowList: setup.numbersFollowList, startFolder: setup.home, paneHeaderHeight: 0),
            listener: wake)
        wake.attach { [weak self] in self?.onWake?() }
    }

    /// The frame source of terminal `id`, over the program started for it; nil when it did not start.
    public func frameSource(for id: TerminalID) -> (any FrameSource)? {
        guard let relay = relays[id], let terminal = workspace.terminal(id: Int64(id)) else { return nil }
        return UniFFIFrameSource(adopting: terminal, relay: relay)
    }

    /// Why a terminal's program did not start, for the window to say.
    public func startFailure(of id: TerminalID) -> String? { failed[id] }

    public func apply(_ action: WorkspaceAction) -> WorkspaceModel.Refusal? {
        handle(workspace.apply(action: WormholeBindings.Action(action)))
    }

    public func drain() { _ = handle(workspace.drain()) }

    public func revisions() -> WorkspaceModel.Revisions { WorkspaceModel.Revisions(workspace.revisions()) }
    public func sidebar() -> WorkspaceModel.SidebarSnapshot { WorkspaceModel.SidebarSnapshot(workspace.sidebar()) }
    public func chrome() -> ChromeState { ChromeState(workspace.chrome()) }

    public func dropPlan(dragged: TerminalID, over: DropOver) -> WorkspaceModel.DropPlan? {
        workspace.dropPlan(dragged: Int64(dragged), over: Over(over)).map(WorkspaceModel.DropPlan.init)
    }

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
            let terminal = try Terminal.startIn(
                program: setup.program, args: setup.arguments, cwd: folder, cols: UInt16(size.cols), rows: UInt16(size.rows),
                cellWidth: size.cellWidth, cellHeight: size.cellHeight, listener: relay)
            relays[id] = relay
            workspace.registerTerminal(id: Int64(id), terminal: terminal)
        } catch {
            failed[id] = "\(error)"
            FileHandle.standardError.write(Data("terminal \(id): cannot start \(setup.program): \(error)\n".utf8))
        }
    }
}
