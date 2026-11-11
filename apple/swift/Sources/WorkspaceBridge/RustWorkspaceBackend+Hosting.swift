// Persistent terminals in the workspace: a terminal starts in a session host of its own when the feature is on, the
// terminals the last run left are attached again at launch, and quitting lets go of every host (or ends them). What the
// core decides (the plan, what a failed attach means, the saved rows, the checkpoints) is Rust's; this carries it out
// against the window. Starting and attaching run off the main actor with the pane blank, as the GPUI app does.

import Foundation
import TerminalCore
import UniFFISource
import WorkspaceModel
import WormholeBindings

extension RustWorkspaceBackend {
    /// Terminal `id`'s program: in a host (a saved terminal attaches to its own), or in the app's process when persistence
    /// is off or no host would start. Started for its tab: its shell has `WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and
    /// `WORMHOLE_BIN`, so agents' hooks and its own `wh` reach this workspace.
    func start(_ id: TerminalID, in folder: String?) {
        let relay = TerminalRelay()
        guard let hosted else {
            if needsApplications { told("move-to-applications", .moveToApplications) }
            do { register(id, try HostConnect.local(request(id, in: folder), relay), relay: relay) } catch {
                failed[id] = "\(error)"
                FileHandle.standardError.write(Data("terminal \(id): cannot start \(setup.program): \(error)\n".utf8))
            }
            return
        }
        let entry = saved.removeValue(forKey: id) ?? (adopting.isEmpty ? nil : adopting.removeFirst())
        switch entry {
        case .attach(let host, _)?:
            connect(id, request(id, in: entry?.folder), attach: host, relay: relay, desk: hosted.desk)
        case .again(let again, let old, let text)?:
            connect(id, request(id, in: again, old: old, text: text), attach: nil, relay: relay, desk: hosted.desk)
        case nil:
            connect(id, request(id, in: folder), attach: nil, relay: relay, desk: hosted.desk)
        }
    }

    /// Begin starting or attaching terminal `id` off the main actor; `connected` takes the result.
    func connect(_ id: TerminalID, _ what: HostedStart, attach host: String?, relay: TerminalRelay, desk: HostedTerminals) {
        relays[id] = relay
        book.tokens += 1
        let token = book.tokens
        book.connecting[id] = token
        let visible = Set(core.shown().map(Int.init)).contains(id)
        book.visible[id] = visible
        startSaving(desk)
        Task { [weak self] in
            let done = await Task.detached(priority: .userInitiated) {
                HostConnect.run(desk: desk, what: what, attach: host, visible: visible, relay: relay)
            }.value
            self?.connected(id, token: token, done)
        }
    }

    private func connected(_ id: TerminalID, token: Int, _ done: Connected) {
        guard book.connecting[id] == token else {
            // Closed or started again meanwhile: what started is nobody's (closing ends its host, as dropping a hosted `Pty` does).
            done.terminal?.close()
            return
        }
        book.connecting[id] = nil
        if book.quitting {
            _ = done.terminal?.detach()
            return
        }
        if let fallback = done.fallback { told("host-fallback", .hostFallback(fallback)) }
        if let terminal = done.terminal {
            register(id, terminal, relay: relays[id])
            // Attached while the window has moved on (or the other way round): the stream follows what is shown now.
            terminal.setVisible(visible: Set(core.shown().map(Int.init)).contains(id))
        } else if let unavailable = done.unavailable {
            let quiet = unavailable.why.isTransient && book.retried.insert(id).inserted
            book.unavailable[id] = (unavailable.why, unavailable.host, !quiet)
            if quiet { retryAfter(id) }
        } else if let failure = done.failure {
            failed[id] = failure
            relays[id] = nil
            FileHandle.standardError.write(Data("terminal \(id): cannot start \(setup.program): \(failure)\n".utf8))
        }
        if book.restoring, book.connecting.isEmpty { finishRestore() }
        onNotice?(.hostChanged(id))
        onWake?()
    }

    /// Every terminal of the launch has its host (or its bar): the rows the last run left under other ids are forgotten.
    private func finishRestore() {
        book.restoring = false
        try? hosted?.desk.finishRestore()
    }

    /// The terminal's program is running: the workspace feeds its row from it, and the view that waited for it draws it.
    func register(_ id: TerminalID, _ terminal: Terminal, relay: TerminalRelay?) {
        if let relay { relays[id] = relay }
        core.registerTerminal(id: Int64(id), terminal: terminal)
        book.unavailable[id] = nil
        book.retried.remove(id)
        sources[id]?.value?.fill(terminal)
    }

    /// Say it, once per run.
    private func told(_ key: String, _ notice: WorkspaceNotice) {
        guard HostBook.told.insert(key).inserted else { return }
        onNotice?(notice)
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
        // The starts and attaches of the launch end off the main actor: the last one to end forgets the old rows.
        if book.connecting.isEmpty { finishRestore() } else { book.restoring = true }
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

    /// `endHosts`: every terminal ends with its host. Otherwise each is let go of and its program runs on, after the
    /// screens are saved for a host that will not be there.
    public func finishQuit(endHosts: Bool) {
        book.quitting = true
        book.saver?.cancel()
        guard let desk = hosted?.desk else { return }
        if !endHosts { _ = desk.saveTexts() }
        desk.finish(endHosts: endHosts)
    }

    public func waited(since: UInt64) -> WorkspaceModel.Waited? {
        let waited = core.waited(sinceMs: since)
        return WorkspaceModel.Waited(words: waited.words, next: .milliseconds(waited.nextInMs))
    }

    /// The screens are kept each minute, so a terminal whose host is gone is drawn from what it showed.
    private func startSaving(_ desk: HostedTerminals) {
        guard book.saver == nil else { return }
        book.saver = Task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(60))
                await Task.detached(priority: .utility) { _ = desk.saveTexts() }.value
            }
        }
    }

    /// The terminals of hosts nobody named come into the Recovered group, never ended by the app.
    private func adopt(_ hosts: [String]) {
        guard !hosts.isEmpty else { return }
        let before = Set(live)
        let selected = sidebar().selected
        openRestored(hosts.map { .attach(hostID: $0, folder: nil) })
        _ = apply(.recoverGroup(live.filter { !before.contains($0) }))
        // Opening the terminals selected each; the person's own stays.
        if let selected { _ = apply(.select(selected)) }
        onWake?()
    }

    private func openRestored(_ entries: [HostedSessions.Entry]) {
        adopting = entries
        for _ in entries { _ = apply(.newTerminal) }
        adopting = []
    }

    func request(_ id: TerminalID, in folder: String?, old: String? = nil, text: String? = nil) -> HostedStart {
        let size = Self.startSize
        let program = programs[id] ?? (setup.program, setup.arguments)
        return HostedStart(
            tab: Int64(id), program: program.0, args: program.1, cwd: folder, cols: UInt16(size.cols),
            rows: UInt16(size.rows), cellWidth: size.cellWidth, cellHeight: size.cellHeight, oldHost: old, savedText: text)
    }
}
