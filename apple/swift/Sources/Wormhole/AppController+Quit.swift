// Closing and quitting ask first when something would end: a terminal in which an agent or a command runs, or any
// terminal when the app quits. The questions are AuxModel's, as native sheets on the window; quitting asks in
// `applicationShouldTerminate`, so the Dock's Quit and logging out ask too.

import AppKit
import AuxModel
import AuxWindows
import PanelModel
import WorkspaceModel

#if WITH_UNIFFI
    import SettingsBridge
#endif

extension AppController {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        let endAll = endsAllOnQuit
        endsAllOnQuit = false
        guard confirmsQuit else { return .terminateNow }
        // The answer decides what happens to the session hosts when the app goes (`applicationWillTerminate`).
        return QuitFlow.shouldTerminate(facts: quitFacts, endAll: endAll, window: window) { [weak self] endHosts in
            self?.endsHostsOnQuit = endHosts
        }
    }

    func quit(endAll: Bool) {
        endsAllOnQuit = endAll
        NSApp.terminate(nil)
    }

    /// Quits without the question: the control folder's `quit` and the idle check run unattended. `endAll`: the hosts end
    /// with their terminals (End All Terminals), else they run on.
    func terminateUnattended(endAll: Bool = false) {
        confirmsQuit = false
        endsHostsOnQuit = endAll
        NSApp.terminate(nil)
    }

    /// What a restart would end, for the updater's question.
    var runningTerminals: [RunningTerminal] {
        allTerminals.filter(\.runsSomething).map {
            RunningTerminal(name: $0.title, agent: $0.agent?.name, status: $0.runStatus, program: $0.command)
        }
    }

    /// Every window's terminals, those folded away in a group too: quitting ends all of them, and the core's list has them all.
    private var allTerminals: [TerminalSummary] { sessions.flatMap { $0.hub.state.all } }

    /// What the quit question is decided from, every window's terminals. A terminal whose session host cannot be shown counts
    /// as running (`needs_close_confirmation`); the terminals of a profile that is not shown run on out of sight, and the question
    /// names that profile with how many (`window::windows::quit_asking`).
    var quitFacts: QuitFacts {
        let rows = sessions.flatMap { session in session.hub.state.all.map { (session: session, row: $0) } }
        let running = rows.filter { $0.row.runsSomething || hostUnavailable($0.row.id, in: $0.session) }
        let counts = HostedCounts(running: running.map(\.row.id), all: rows.map(\.row.id)) { id in
            self.session(holding: id)?.hub.store.hosting(of: id)
        }
        var elsewhere: [(name: String, count: Int)] = []
        // Out of sight only: a window of another profile that is open in front of the person is counted as any window.
        let names = profileNames()
        for (session, _) in running where session.parked {
            let name = names[session.windows.profile] ?? session.windows.profile
            if let at = elsewhere.firstIndex(where: { $0.name == name }) { elsewhere[at].count += 1 } else { elsewhere.append((name, 1)) }
        }
        return QuitFacts(
            running: running.map { StillRunning.Fact(status: $0.row.agentStatus, commandRunning: $0.row.command != nil) },
            programs: running.compactMap { $0.row.command }, otherProfiles: elsewhere, hostedRunning: counts.hostedRunning,
            localRunning: counts.localRunning, anyHosted: counts.anyHosted)
    }

    private func hostUnavailable(_ id: TerminalID, in session: WindowSession) -> Bool {
        if case .unavailable? = session.hub.store.hostProblem(of: id) { true } else { false }
    }

    /// The profiles' names as the person gave them, by id (read once).
    private func profileNames() -> [String: String] {
        #if WITH_UNIFFI
            Dictionary(
                ((try? (settingsModel.host as? RustSettingsHost)?.profiles()) ?? []).map { ($0.id, $0.name) }, uniquingKeysWith: { first, _ in first })
        #else
            [:]
        #endif
    }

    /// Closes a terminal (of `session`, else the window that holds it), asking first when an agent or a command runs in it. One
    /// the command line asked to close (`wh close`) is always asked about: it never closes on a script's say (`CloseOrigin::Control`).
    func closeTerminal(_ id: TerminalID, in session: WindowSession? = nil, fromCommandLine: Bool = false, then done: @escaping () -> Void = {}) {
        let session = session ?? self.session(holding: id) ?? front
        // From the core's list of every terminal: one folded away in a group has no row, and what is at stake is the core's fact.
        guard let terminal = session.hub.state.all.first(where: { $0.id == id }) else { return done() }
        // A terminal whose host runs and cannot be shown: closing drops the row and with it the host's id, so ask first
        // whether the host ends with it or is left running (`window/host_close.rs`).
        if case .unavailable? = session.hub.store.hostProblem(of: id) {
            return askToEndHost(of: terminal, in: session, then: done)
        }
        Task {
            let extra = await worktreeClose(of: [terminal], in: session, group: false)
            guard terminal.runsSomething || fromCommandLine || extra != nil else {
                finishClosing(id, in: session)
                return done()
            }
            var detail = terminal.runsSomething || fromCommandLine
                ? CloseQuestions.session(
                    agent: terminal.agent?.name, stakes: Self.stakes(of: [terminal], in: session), status: terminal.runStatus,
                    commandRunning: terminal.command != nil, programs: terminal.command.map { [$0] } ?? [])
                : ""
            if fromCommandLine { detail += " " + AuxText.controlCloseAsked }
            if let extra { detail = detail.isEmpty ? extra.note : detail + " " + extra.note }
            let question = Question(
                title: CloseQuestions.sessionTitle(terminal.title), detail: detail, confirm: "Close", alsoButtons: extra?.also.map { [$0] } ?? [])
            switch await QuestionSheet(question).ask(in: session.window) {
            case .confirm: finishClosing(id, in: session)
            case .also:
                finishClosing(id, in: session)
                for made in extra?.remove ?? [] { Task { await session.hub.panel.worktrees.remove(made) } }
            case .cancel: break
            }
            done()
        }
    }

    private func askToEndHost(of terminal: TerminalSummary, in session: WindowSession, then done: @escaping () -> Void) {
        let question = Question(
            title: CloseQuestions.sessionTitle(terminal.title),
            detail: String(
                localized: "hosts-close-detail",
                defaultValue: "Its session host is still running, but this app can’t show it. Ending the host ends the program in it; leaving it running keeps it in the Session Hosts window."),
            confirm: String(localized: "hosts-close-end", defaultValue: "End the Host"),
            alsoButtons: [String(localized: "hosts-close-leave", defaultValue: "Leave It Running")])
        Task {
            switch await QuestionSheet(question).ask(in: session.window) {
            // Closes the terminal when the host ended, and the bar says why when it did not.
            case .confirm: session.hub.endHost(terminal.id, hard: false)
            case .also: finishClosing(terminal.id, in: session)
            case .cancel: break
            }
            done()
        }
    }

    /// Closes a group's terminals, asking first when any of them works or runs something.
    func closeGroup(_ id: GroupID, in session: WindowSession? = nil) {
        let session = session ?? front
        guard let section = session.hub.state.sections.first(where: { $0.head?.id == id }), let head = section.head else { return }
        // A folded group lists no rows: the core counts what works in it, and says all of it is to end.
        let members = session.hub.state.all.filter { $0.group == id }
        let running = members.filter(\.runsSomething)
        Task {
            let extra = await worktreeClose(of: members, in: session, group: true)
            guard section.workingCount > 0 || !running.isEmpty || extra != nil else { return finishClosingGroup(id, in: session) }
            var detail = section.workingCount > 0 || !running.isEmpty
                ? CloseQuestions.group(
                    terminals: members.count,
                    running: running.map { StillRunning.Fact(status: $0.agentStatus, commandRunning: $0.command != nil) },
                    programs: running.compactMap(\.command), stakes: Self.stakes(of: section.rows.map(\.terminal), in: session))
                : ""
            if let extra { detail = detail.isEmpty ? extra.note : detail + " " + extra.note }
            let question = Question(
                title: CloseQuestions.groupTitle(head.name), detail: detail, confirm: "Close Group", alsoButtons: extra?.also.map { [$0] } ?? [])
            switch await QuestionSheet(question).ask(in: session.window) {
            case .confirm: finishClosingGroup(id, in: session)
            case .also:
                finishClosingGroup(id, in: session)
                for made in extra?.remove ?? [] { Task { await session.hub.panel.worktrees.remove(made) } }
            case .cancel: break
            }
        }
    }

    /// Closing a window while something runs in it: the question, and the window closes only on a yes.
    func askToClose(_ session: WindowSession, running: [TerminalSummary]) {
        let question = Question(
            title: "Close this window?",
            detail: CloseQuestions.window(
                terminals: session.hub.state.live.count,
                running: running.map { StillRunning.Fact(status: $0.agentStatus, commandRunning: $0.command != nil) },
                programs: running.compactMap(\.command), stakes: Self.stakes(of: session.hub.state.sections.flatMap(\.rows).map(\.terminal), in: session)),
            confirm: "Close Window")
        Task {
            // `close()` does not ask the window again: the question was this.
            if case .confirm = await QuestionSheet(question).ask(in: session.window) { session.window?.close() }
        }
    }

    /// What closing these terminals discards of their notes and tasks, in the core's words, while the Notes tab is on
    /// (`close_stakes`: each feature that is on says what it holds).
    static func stakes(of terminals: [TerminalSummary], in session: WindowSession) -> [String] {
        guard session.hub.panelTabs.contains(.notes) else { return [] }
        return NoteRules.closeStake(terminals.reduce(0) { $0 + $1.noteCount }).map { [$0] } ?? []
    }

    private func finishClosingGroup(_ id: GroupID, in session: WindowSession) {
        session.hub.send(.closeGroup(id))
        if session.hub.state.live.isEmpty { session.window?.close() }
    }

    func finishClosing(_ id: TerminalID, in session: WindowSession) {
        session.hub.send(.close(id))
        if session.hub.state.live.isEmpty { session.window?.close() }
    }
}

extension SidebarRow {
    /// The command a shell runs, when the row's second line is one (the core words it): for the ambient words only; what a
    /// close or a quit would end is read from the core's own facts (`TerminalSummary.command`).
    var runningCommand: String? { secondLine.isCommand ? secondLine.text : nil }
}

extension TerminalSummary {
    /// An agent works or waits, or a program runs in front: closing this ends something (`close_needs_confirmation`).
    var runsSomething: Bool { activity == .working || activity == .needsInput || command != nil }

    /// What the quit question may call an agent's: the real status where an agent is in the terminal, else idle. Any command makes a
    /// terminal busy (its output is the signal), and `top` is not an agent: the question names what runs through the programs
    /// (`Session::agent_status`).
    var agentStatus: RunningTerminal.Status { agent == nil ? .idle : runStatus }

    var runStatus: RunningTerminal.Status {
        switch activity {
        case .needsInput: .attention
        case .working: .busy
        default: .idle
        }
    }
}
