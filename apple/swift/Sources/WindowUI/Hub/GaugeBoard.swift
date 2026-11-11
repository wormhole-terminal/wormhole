// What each terminal's coding agent last said about itself on its status line: the model it runs and how full its
// context window is (core `usage::Gauge`; the title row's chip). The core's rows hold neither, so the app keeps the
// latest word per terminal here. A gauge belongs to the agent that made it: once another agent (or none) is in the
// terminal, the old numbers are not shown, as the GPUI app forgets them when the shell has its terminal back.

import Observation
import PanesModel
import WorkspaceModel

@MainActor @Observable
public final class GaugeBoard {
    private struct Entry {
        var agent: AgentID?
        var gauge: GaugeFact
    }

    private var told: [TerminalID: Entry] = [:]

    public init() {}

    /// `terminal`'s `agent` (its command, `claude`) reported `gauge`.
    public func tell(terminal: TerminalID, agent: String, gauge: GaugeFact) {
        told[terminal] = Entry(agent: AgentID(command: agent), gauge: gauge)
    }

    /// Forget the gauges of terminals whose shell has them back (no agent, nothing else in front), as the GPUI app's
    /// `forget_gone_gauges`; while a program is merely not recognised yet (an agent just started) its gauge is kept.
    public func forgetGone(_ terminals: [TerminalSummary]) {
        for terminal in terminals where told[terminal.id] != nil && terminal.agent == nil && terminal.runningCommand == nil {
            told[terminal.id] = nil
        }
    }

    /// The gauge of the agent that is in `terminal` now, if it ever reported one.
    public func gauge(for terminal: TerminalSummary) -> GaugeFact? {
        guard let entry = told[terminal.id], let agent = terminal.agent, entry.agent == agent else { return nil }
        return entry.gauge
    }
}
