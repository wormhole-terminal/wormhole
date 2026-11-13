// Letting go of this window's hosted terminals and attaching them again, one window at a time (GPUI's devctl `detach` and
// `reattach`, `Workspace::detach_hosts` / `reattach_hosts`): what the app does to every window as it quits and launches,
// through the same calls (`Terminal.detach`, which `HostedTerminals.finish` makes for each, and `connect`, the launch's
// attach). The views stay: a terminal let go of keeps its last screen until it is attached again.

import Foundation
import UniFFISource
import WorkspaceModel
import WormholeBindings

extension RustWorkspaceBackend {
    /// Lets go of every hosted terminal of this window: the programs go on running in their hosts. How many were let go of.
    public func detachHosts() -> Int {
        var count = 0
        for id in live where book.connecting[id] == nil {
            guard let terminal = core.terminal(id: Int64(id)), terminal.detach() else { continue }
            count += 1
            onNotice?(.hostChanged(id))
        }
        if count > 0 { onWake?() }
        return count
    }

    /// Attaches the terminals let go of to their hosts again, as a launch does (on screen with a repaint, the rest
    /// headless; a host that is gone gets a new terminal in its place). Their ids, in the window's order.
    public func reattachHosts() -> [TerminalID] {
        guard let hosted else { return [] }
        var ids: [TerminalID] = []
        for id in live where book.connecting[id] == nil {
            guard let terminal = core.terminal(id: Int64(id)), case .detached(let host) = HostingState(terminal.hosting()) else { continue }
            // The view's source keeps listening through the relay it has; the terminal that comes fills it (`register`).
            connect(id, request(id, in: core.cwdOf(id: Int64(id))), attach: host, relay: relays[id] ?? TerminalRelay(), desk: hosted.desk)
            onNotice?(.hostChanged(id))
            ids.append(id)
        }
        return ids
    }
}
