// Closing and quitting ask first when something would end: a terminal in which an agent or a command runs, or any
// terminal when the app quits. The questions are AuxModel's, as native sheets on the window; quitting asks in
// `applicationShouldTerminate`, so the Dock's Quit and logging out ask too.

import AppKit
import AuxModel
import AuxWindows
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
        allRows.filter(\.runsSomething).map {
            RunningTerminal(name: $0.terminal.title, agent: $0.terminal.agent?.name, status: $0.terminal.runStatus, program: $0.runningCommand)
        }
    }

    /// Every window's rows: quitting ends the terminals of all of them.
    private var allRows: [SidebarRow] { sessions.flatMap { $0.hub.state.sections.flatMap(\.rows) } }

    /// What the quit question is decided from, every window's terminals. A terminal whose session host cannot be shown counts
    /// as running (`needs_close_confirmation`); the terminals of a profile that is not shown run on out of sight, and the question
    /// names that profile with how many (`window::windows::quit_asking`).
    var quitFacts: QuitFacts {
        let rows = sessions.flatMap { session in session.hub.state.sections.flatMap(\.rows).map { (session: session, row: $0) } }
        let running = rows.filter { $0.row.runsSomething || hostUnavailable($0.row.id, in: $0.session) }
        let counts = HostedCounts(running: running.map(\.row.id), all: rows.map(\.row.id)) { id in
            self.session(holding: id)?.hub.store.hosting(of: id)
        }
        var elsewhere: [(name: String, count: Int)] = []
        for (session, _) in running where session.windows.profile != windowsHost.profile {
            let name = profileName(session.windows.profile)
            if let at = elsewhere.firstIndex(where: { $0.name == name }) { elsewhere[at].count += 1 } else { elsewhere.append((name, 1)) }
        }
        return QuitFacts(
            running: running.map { StillRunning.Fact(status: $0.row.terminal.agentStatus, commandRunning: $0.row.runningCommand != nil) },
            programs: running.compactMap { $0.row.runningCommand }, otherProfiles: elsewhere, hostedRunning: counts.hostedRunning,
            localRunning: counts.localRunning, anyHosted: counts.anyHosted)
    }

    private func hostUnavailable(_ id: TerminalID, in session: WindowSession) -> Bool {
        if case .unavailable? = session.hub.store.hostProblem(of: id) { true } else { false }
    }

    /// A profile's name as the person gave it, else its id.
    private func profileName(_ id: String) -> String {
        #if WITH_UNIFFI
            ((try? (settingsModel.host as? RustSettingsHost)?.profiles()) ?? []).first { $0.id == id }?.name ?? id
        #else
            id
        #endif
    }

    /// Closes a terminal (of `session`, else the window that holds it), asking first when an agent or a command runs in it. One
    /// the command line asked to close (`wh close`) is always asked about: it never closes on a script's say (`CloseOrigin::Control`).
    func closeTerminal(_ id: TerminalID, in session: WindowSession? = nil, fromCommandLine: Bool = false, then done: @escaping () -> Void = {}) {
        let session = session ?? self.session(holding: id) ?? front
        guard let row = session.hub.state.sections.flatMap(\.rows).first(where: { $0.id == id }) else { return done() }
        let terminal = row.terminal
        // A terminal whose host runs and cannot be shown: closing drops the row and with it the host's id, so ask first
        // whether the host ends with it or is left running (`window/host_close.rs`).
        if case .unavailable? = session.hub.store.hostProblem(of: id) {
            return askToEndHost(of: terminal, in: session, then: done)
        }
        guard row.runsSomething || fromCommandLine else {
            finishClosing(id, in: session)
            return done()
        }
        let detail = CloseQuestions.session(
            agent: terminal.agent?.name, stakes: [], status: terminal.runStatus,
            commandRunning: row.runningCommand != nil, programs: row.runningCommand.map { [$0] } ?? [])
        let question = Question(
            title: CloseQuestions.sessionTitle(terminal.title),
            detail: fromCommandLine ? detail + " " + AuxText.controlCloseAsked : detail, confirm: "Close")
        Task {
            if case .confirm = await QuestionSheet(question).ask(in: session.window) { finishClosing(id, in: session) }
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
        let running = section.rows.filter(\.runsSomething)
        guard section.workingCount > 0 || !running.isEmpty else { return finishClosingGroup(id, in: session) }
        let question = Question(
            title: CloseQuestions.groupTitle(head.name),
            detail: CloseQuestions.group(
                terminals: section.terminalCount,
                running: running.map { StillRunning.Fact(status: $0.terminal.agentStatus, commandRunning: $0.runningCommand != nil) },
                programs: running.compactMap(\.runningCommand), stakes: []),
            confirm: "Close Group")
        Task {
            if case .confirm = await QuestionSheet(question).ask(in: session.window) { finishClosingGroup(id, in: session) }
        }
    }

    /// Closing a window while something runs in it: the question, and the window closes only on a yes.
    func askToClose(_ session: WindowSession, running: [SidebarRow]) {
        let question = Question(
            title: "Close this window?",
            detail: CloseQuestions.window(
                terminals: session.hub.state.live.count,
                running: running.map { StillRunning.Fact(status: $0.terminal.agentStatus, commandRunning: $0.runningCommand != nil) },
                programs: running.compactMap(\.runningCommand), stakes: []),
            confirm: "Close Window")
        Task {
            // `close()` does not ask the window again: the question was this.
            if case .confirm = await QuestionSheet(question).ask(in: session.window) { session.window?.close() }
        }
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
    /// The command a shell runs, when the row's second line is one (the core words it).
    var runningCommand: String? { secondLine.isCommand ? secondLine.text : nil }

    /// An agent works or waits, or a command that ends by itself runs: closing this ends something.
    var runsSomething: Bool { terminal.activity == .working || terminal.activity == .needsInput || runningCommand != nil }
}

extension TerminalSummary {
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
