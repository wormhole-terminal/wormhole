// Persistent terminals in the workspace: a terminal starts in a session host of its own when the feature is on, the
// terminals the last run left are attached again at launch, and quitting lets go of every host (or ends them). What the
// core decides (the plan, what a failed attach means, the saved rows) is Rust's; this carries it out against the window.

import Foundation
import TerminalCore
import UniFFISource
import WorkspaceModel
import WormholeBindings

extension RustWorkspaceBackend {
    /// The program of terminal `id`, started where it should run: in a host (a saved terminal attaches to its own), or in
    /// the app's process when persistence is off or no host would start. Started for its tab: its shell has
    /// `WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and `WORMHOLE_BIN`, so agents' hooks and its own `wh` reach this workspace.
    func makeTerminal(_ id: TerminalID, in folder: String?, relay: TerminalRelay) throws -> Terminal {
        guard let hosted else { return try startLocal(id, in: folder, relay: relay) }
        let entry = saved.removeValue(forKey: id) ?? (adopting.isEmpty ? nil : adopting.removeFirst())
        let what = request(id, in: entry?.folder ?? folder)
        if case .attach(let host, _)? = entry {
            do { return try hosted.desk.attach(hostId: host, what: what, visible: true, listener: relay) } catch HostingError.Gone {
                // Its program went with its host: a new terminal where it was.
            }
        }
        do {
            return try hosted.desk.start(what: what, listener: relay)
        } catch HostingError.Start(let reason), HostingError.NoProgram(let reason) {
            hostingFallback = hostingFallback ?? HostingWords.fallback(reason)
            FileHandle.standardError.write(Data("terminal \(id): \(HostingWords.fallback(reason))\n".utf8))
            return try startLocal(id, in: folder, relay: relay)
        }
    }

    /// Brings back what the last run left in this window: from the window's record (the terminals keep their ids, each
    /// attached to its own host or started again where its host is gone), or, for a workspace that is no window of the
    /// app's, the saved terminals one by one under new ids. Then, off the main thread, the hosts nobody named. How many
    /// terminals came back; 0 means the caller opens its own.
    @discardableResult
    public func restoreSaved() -> Int {
        let before = live.count
        guard let hosted else {
            _ = handle(core.restore())
            return live.count - before
        }
        let plan = hosted.launchPlan()
        saved = Dictionary(plan.map { ($0.terminal, $0.entry) }, uniquingKeysWith: { first, _ in first })
        _ = handle(core.restore())
        // A workspace with no record of its own (not among the app's windows) opens what the hosts saved, in order.
        if live.count == before, !plan.isEmpty {
            openRestored(plan.map { $0.entry })
            if let first = live.first { _ = apply(.select(first)) }
        }
        saved = [:]
        try? hosted.desk.finishRestore()
        // Opened behind the store's back: it pulls what changed.
        onWake?()
        if hosted.adoptsOrphans {
            let desk = hosted.desk
            Task.detached { [weak self] in
                let orphans = desk.adoptableOrphans().map(\.hostId)
                await MainActor.run { self?.adopt(orphans) }
            }
        }
        return live.count - before
    }

    /// Fill an empty window from its record (the protocol's `restore`).
    public func restore() { restoreSaved() }

    public func hosting(of id: TerminalID) -> HostingState? {
        guard hosted != nil else { return nil }
        return core.terminal(id: Int64(id)).map { HostingState($0.hosting()) } ?? .local
    }

    /// `endHosts`: every terminal ends with its host. Otherwise each is let go of and its program runs on.
    public func finishQuit(endHosts: Bool) { hosted?.desk.finish(endHosts: endHosts) }

    /// The terminals of hosts nobody named come into a Recovered group, never ended by the app.
    private func adopt(_ hosts: [String]) {
        guard !hosts.isEmpty else { return }
        let before = Set(live)
        openRestored(hosts.map { .attach(hostID: $0, folder: nil) })
        let ids = live.filter { !before.contains($0) }
        _ = apply(.newGroup(ids))
        let made = Set(ids)
        let group = sidebar().sections.first { section in
            section.head != nil && Set(section.rows.map(\.id)) == made
        }?.head
        if let group { _ = apply(.renameGroup(group.id, HostingWords.recoveredGroup)) }
        onWake?()
    }

    private func openRestored(_ entries: [HostedSessions.Entry]) {
        adopting = entries
        for _ in entries { _ = apply(.newTerminal) }
        adopting = []
    }

    private func request(_ id: TerminalID, in folder: String?) -> HostedStart {
        let size = Self.startSize
        return HostedStart(
            tab: Int64(id), program: setup.program, args: setup.arguments, cwd: folder, cols: UInt16(size.cols),
            rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight)
    }

    private func startLocal(_ id: TerminalID, in folder: String?, relay: TerminalRelay) throws -> Terminal {
        let what = request(id, in: folder)
        return try Terminal.startForTab(
            tab: what.tab, program: what.program, args: what.args, cwd: what.cwd, cols: what.cols, rows: what.rows,
            cellWidth: what.cellWidth, cellHeight: what.cellHeight, listener: relay)
    }
}
