// Closing and quitting ask first when something would end: a terminal in which an agent or a command runs, or any
// terminal when the app quits. The questions are AuxModel's, as native sheets on the window; quitting asks in
// `applicationShouldTerminate`, so the Dock's Quit and logging out ask too.

import AppKit
import AuxModel
import AuxWindows
import WorkspaceModel

extension AppController {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        let endAll = endsAllOnQuit
        endsAllOnQuit = false
        guard confirmsQuit else { return .terminateNow }
        return QuitFlow.shouldTerminate(facts: quitFacts, endAll: endAll, window: window) { _ in }
    }

    func quit(endAll: Bool) {
        endsAllOnQuit = endAll
        NSApp.terminate(nil)
    }

    /// Quits without the question: the control folder's `quit` and the idle check run unattended.
    func terminateUnattended() {
        confirmsQuit = false
        NSApp.terminate(nil)
    }

    private var quitFacts: QuitFacts {
        let running = hub.state.terminals.filter(\.runsSomething)
        return QuitFacts(
            running: running.map { StillRunning.Fact(status: $0.runStatus, commandRunning: $0.runningCommand != nil) },
            programs: running.compactMap { $0.runningCommand }, localRunning: running.count)
    }

    /// Closes a terminal, asking first when an agent or a command runs in it.
    func closeTerminal(_ id: TerminalID) {
        guard let terminal = hub.state.terminals.first(where: { $0.id == id }) else { return }
        guard terminal.runsSomething else { return finishClosing(id) }
        let question = Question(
            title: CloseQuestions.sessionTitle(terminal.title),
            detail: CloseQuestions.session(
                agent: terminal.agent?.name, stakes: [], status: terminal.runStatus,
                commandRunning: terminal.runningCommand != nil, programs: terminal.runningCommand.map { [$0] } ?? []),
            confirm: "Close")
        Task {
            if case .confirm = await QuestionSheet(question).ask(in: window) { finishClosing(id) }
        }
    }

    private func finishClosing(_ id: TerminalID) {
        hub.send(.close(id))
        if hub.state.terminals.isEmpty { window?.close() }
    }
}

extension TerminalSummary {
    /// An agent works or waits, or a command that ends by itself runs: closing this ends something.
    var runsSomething: Bool { activity == .working || activity == .needsInput || runningCommand != nil }

    var runStatus: RunningTerminal.Status {
        switch activity {
        case .needsInput: .attention
        case .working: .busy
        default: .idle
        }
    }
}
