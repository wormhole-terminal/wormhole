// What still runs, listed the way Terminal's question lists it (`session::still_running`, `running_list`): an
// agent waiting or working, then the programs by name, each once, a long command cut short and a long list ending
// in "and N more".

import Foundation

public enum StillRunning {
    private static let programMax = 40
    private static let programsListed = 4

    public static func shortened(_ name: String) -> String {
        name.count > programMax ? String(name.prefix(programMax)) + "…" : name
    }

    /// The programs that end, "claude, npm run dev", each once and in order; `nil` for none.
    public static func list(_ programs: [String]) -> String? {
        var names: [String] = []
        for program in programs where !names.contains(program) { names.append(program) }
        guard !names.isEmpty else { return nil }
        var list = names.prefix(programsListed).map(shortened).joined(separator: ", ")
        if names.count > programsListed { list += " " + AuxText.andMore(names.count - programsListed) }
        return list
    }

    /// Whether each running terminal asks (its lead status and whether a command runs).
    public struct Fact: Equatable, Sendable {
        public var status: RunningTerminal.Status
        public var commandRunning: Bool
        public init(status: RunningTerminal.Status, commandRunning: Bool) { (self.status, self.commandRunning) = (status, commandRunning) }
    }

    /// Each sentence after a space, ready to append to a question's detail.
    public static func detail(_ running: [Fact], programs: [String]) -> String {
        var detail = ""
        if running.contains(where: { $0.status == .attention }) {
            detail += " " + AuxText.agentWaiting
        } else if running.contains(where: { $0.status == .busy }) {
            detail += " " + AuxText.agentWorking
        }
        if let list = list(programs) {
            detail += " " + AuxText.stillRunning(list)
        } else if detail.isEmpty, running.contains(where: \.commandRunning) {
            detail += " " + AuxText.commandRunning
        }
        return detail
    }
}
