// The question before restarting to use an installed update (`update::restart`): it restarts at once when nothing
// would end; otherwise a sheet lists what runs and offers to wait ("Restart When Idle", "Later").

import Foundation

public enum RestartQuestion {
    private static let listed = 6

    static func what(_ running: RunningTerminal) -> String {
        switch (running.agent, running.status) {
        case (let agent?, .busy): return "\(agent) is working"
        case (let agent?, .attention): return "\(agent) is waiting for an answer"
        default:
            if let program = running.program { return StillRunning.shortened(program) }
            switch running.status {
            case .busy: return "still working"
            case .attention: return "waiting for you"
            case .idle: return "a command is running"
            }
        }
    }

    /// "api — claude is working": the name is dropped when what it does already starts with it.
    static func line(_ running: RunningTerminal) -> String {
        let name = StillRunning.shortened(running.name), what = what(running)
        let (lowerName, lowerWhat) = (name.lowercased(), what.lowercased())
        let repeats = lowerWhat == lowerName || (lowerWhat.hasPrefix(lowerName + " "))
        return repeats ? what : "\(name) — \(what)"
    }

    public static func detail(_ running: [RunningTerminal]) -> String {
        let shown = running.count > listed + 1 ? listed : running.count
        var lines = running.prefix(shown).map(line)
        if shown < running.count { lines.append("and \(running.count - shown) more") }
        return lines.isEmpty ? AuxText.restartNote : lines.joined(separator: "\n") + "\n\n" + AuxText.restartNote
    }

    /// `nil` when nothing would end: the restart needs no question.
    public static func question(_ running: [RunningTerminal]) -> Question? {
        guard !running.isEmpty else { return nil }
        return Question(
            title: AuxText.restartTitle, detail: detail(running), confirm: AuxText.restart,
            alsoButtons: [AuxText.restartWhenIdle], cancel: AuxText.later)
    }
}
