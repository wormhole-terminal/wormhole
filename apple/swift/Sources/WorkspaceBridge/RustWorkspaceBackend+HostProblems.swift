// What the bar over a pane says of its terminal's session host, and what its buttons do (`terminal_view/host_problem.rs`):
// a host that runs and cannot be shown (try again, a new terminal here, end it), one that stopped answering (wait, end it)
// and the shell that ended with its host. Which terminals a host feeds follows what is on screen.

import Foundation
import UniFFISource
import WorkspaceModel
import WormholeBindings

/// How long before an attach that failed in a way that may pass is tried once more, before the bar is shown.
private let retryDelay = Duration.milliseconds(1500)

extension RustWorkspaceBackend {
    public func hostProblem(of id: TerminalID) -> HostProblem? {
        if let unavailable = book.unavailable[id], unavailable.shown, book.connecting[id] == nil {
            return .unavailable(unavailable.why)
        }
        guard let terminal = core.terminal(id: Int64(id)), terminal.hostId() != nil else { return nil }
        if case .lost = terminal.hosting() { return .lost }
        return terminal.hostNotResponding() == nil ? nil : .notResponding
    }

    public func hosting(of id: TerminalID) -> HostingState? {
        guard hosted != nil else { return nil }
        if book.connecting[id] != nil { return .connecting }
        if let unavailable = book.unavailable[id] { return .unavailable(hostID: unavailable.host) }
        return core.terminal(id: Int64(id)).map { HostingState($0.hosting()) } ?? .local
    }

    /// "Try again": the host's own place is asked again, the pane blank until it answers.
    public func reconnect(_ id: TerminalID) {
        guard let hosted, let unavailable = book.unavailable[id], let relay = relays[id] else { return }
        book.unavailable[id] = nil
        connect(id, request(id, in: core.cwdOf(id: Int64(id))), attach: unavailable.host, relay: relay, desk: hosted.desk)
        onNotice?(.hostChanged(id))
    }

    /// Once more, a moment later, before the bar is shown for a host that was only busy or slow.
    func retryAfter(_ id: TerminalID) {
        Task { [weak self] in
            try? await Task.sleep(for: retryDelay)
            self?.reconnect(id)
        }
    }

    public func endHost(of id: TerminalID, hard: Bool, done: @escaping @MainActor (String?) -> Void) {
        guard let hosted, let host = book.unavailable[id]?.host ?? core.terminal(id: Int64(id))?.hostId() else { return done(nil) }
        let desk = hosted.desk
        Task { [weak self] in
            let why = await Task.detached { desk.endHost(hostId: host, hard: hard) }.value
            if why == nil { self?.onNotice?(.hostEnded(id)) }
            done(why)
        }
    }

    /// A new program in this terminal's place (a new host with a new id when persistent terminals are on): the one that
    /// ended, or whose host could not be shown, is let go of; its host, if it still runs, stays in the Session Hosts window.
    public func startAgain(_ id: TerminalID) {
        guard live.contains(id) else { return }
        let folder = core.cwdOf(id: Int64(id))
        let old = core.terminal(id: Int64(id))
        // The old view's source would end the program it holds; the new one is made from the new terminal.
        sources[id]?.value?.detach()
        sources[id] = nil
        book.connecting[id] = nil
        book.unavailable[id] = nil
        old?.close()
        start(id, in: folder)
        onNotice?(.hostChanged(id))
    }

    /// The terminals on screen are fed by their hosts; the rest stay headless until they are shown.
    func syncVisibility() {
        guard hosted != nil else { return }
        let shown = Set(core.shown().map(Int.init))
        for id in live {
            let on = shown.contains(id)
            guard book.connecting[id] == nil, book.visible[id] != on, let terminal = core.terminal(id: Int64(id)) else { continue }
            book.visible[id] = on
            terminal.setVisible(visible: on)
        }
    }
}
