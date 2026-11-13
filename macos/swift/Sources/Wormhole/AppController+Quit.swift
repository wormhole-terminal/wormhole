// Closing and quitting ask first when something would end. Which terminal asks is the core's rule (`asksToClose`,
// `workspace::asks_before_closing`), read for every terminal a window holds (`state.all`), listed in the sidebar or not. The questions are AuxModel's, as native sheets on the window; quitting asks in
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
        sessions.flatMap(asking).map { RunningTerminal(name: $0.title, agent: $0.agent?.name, status: $0.runStatus, program: $0.command) }
    }

    /// The terminals of `session` that closing would cut off, folded away or filtered out of the list too. One whose
    /// session host runs and cannot be shown counts (`needs_close_confirmation`).
    func asking(in session: WindowSession) -> [TerminalSummary] {
        session.hub.state.all.filter { $0.asksToClose || hostUnavailable($0.id, in: session) }
    }

    /// What the quit question is decided from, every window's terminals. A terminal whose session host cannot be shown counts
    /// as running (`needs_close_confirmation`); the terminals of a profile that is not shown run on out of sight, and the question
    /// names that profile with how many (`window::windows::quit_asking`).
    var quitFacts: QuitFacts {
        let running = sessions.flatMap { session in asking(in: session).map { (session: session, terminal: $0) } }
        let counts = HostedCounts(running: running.map(\.terminal.id), all: sessions.flatMap { $0.hub.state.all.map(\.id) }) { id in
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
            running: running.map(\.terminal.runningFact), programs: running.compactMap(\.terminal.command), otherProfiles: elsewhere, hostedRunning: counts.hostedRunning,
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
        // One question at a time: a second ⌘W while one is up is dropped (`closing_session_prompt`).
        guard !session.asking, let terminal = session.hub.state.all.first(where: { $0.id == id }) else { return done() }
        // A terminal whose host runs and cannot be shown: closing drops the row and with it the host's id, so ask first
        // whether the host ends with it or is left running (`window/host_close.rs`).
        if case .unavailable? = session.hub.store.hostProblem(of: id) {
            return askToEndHost(of: terminal, in: session, then: done)
        }
        session.asking = true
        Task {
            defer { session.asking = false }
            let extra = await worktreeClose(of: [terminal], in: session, group: false)
            guard terminal.asksToClose || fromCommandLine || extra != nil else {
                finishClosing(id, in: session)
                return done()
            }
            // The agent the question names is not listed again among the programs.
            let programs = terminal.runStatus == .idle ? terminal.command.map { [$0] } ?? [] : []
            var detail = terminal.asksToClose || fromCommandLine
                ? CloseQuestions.session(
                    agent: terminal.agent?.name, stakes: Self.stakes(of: [terminal], in: session), status: terminal.runStatus,
                    commandRunning: terminal.command != nil, programs: programs)
                : ""
            if fromCommandLine { detail += " " + AuxText.controlCloseAsked }
            if let extra { detail = detail.isEmpty ? extra.note : detail + " " + extra.note }
            let question = Question(
                title: CloseQuestions.sessionTitle(terminal.title), detail: detail, confirm: AuxText.closeConfirmSession, alsoButtons: extra?.also.map { [$0] } ?? [])
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
        let question = CloseQuestions.unavailableHost(name: terminal.title)
        session.asking = true
        Task {
            defer { session.asking = false }
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
        guard !session.asking, let head = session.hub.state.groups.first(where: { $0.id == id }) else { return }
        // A folded group lists no rows: its terminals are read from every terminal the window holds.
        let members = session.hub.state.all.filter { $0.group == id }
        let running = asking(in: session).filter { $0.group == id }
        session.asking = true
        Task {
            defer { session.asking = false }
            let extra = await worktreeClose(of: members, in: session, group: true)
            guard !running.isEmpty || extra != nil else { return finishClosingGroup(id, in: session) }
            var detail = !running.isEmpty
                ? CloseQuestions.group(
                    terminals: members.count, running: running.map(\.runningFact), programs: running.compactMap(\.command),
                    stakes: Self.stakes(of: members, in: session))
                : ""
            if let extra { detail = detail.isEmpty ? extra.note : detail + " " + extra.note }
            let question = Question(
                title: CloseQuestions.groupTitle(head.name), detail: detail, confirm: AuxText.closeConfirmGroup, alsoButtons: extra?.also.map { [$0] } ?? [])
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
    func askToClose(_ session: WindowSession) {
        guard !session.asking else { return }
        let running = asking(in: session)
        let question = Question(
            title: AuxText.closeWindowTitle,
            detail: CloseQuestions.window(
                terminals: session.hub.state.live.count,
                running: running.map(\.runningFact), programs: running.compactMap(\.command), stakes: Self.stakes(of: session.hub.state.all, in: session)),
            confirm: AuxText.closeConfirmWindow)
        session.asking = true
        Task {
            defer { session.asking = false }
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

extension TerminalSummary {
    /// What the quit question may call an agent's: the real status where an agent is in the terminal, else idle. Any command makes a
    /// terminal busy (its output is the signal), and `top` is not an agent: the question names what runs through the programs
    /// (`Session::agent_status`).
    var agentStatus: RunningTerminal.Status { agent == nil ? .idle : runStatus }

    /// The core's status of the terminal (`run`), as the questions take it.
    var runStatus: RunningTerminal.Status {
        switch run {
        case .attention: .attention
        case .busy: .busy
        case .idle: .idle
        }
    }

    var runningFact: StillRunning.Fact { StillRunning.Fact(status: agentStatus, commandRunning: command != nil) }
}
