// A sidebar terminal of any window, as the ambient surfaces (status item, Dock menu, quick panel) show it
// (`notify::Terminal`): facts as values, so the window feeds them and the surfaces never read a terminal.

import Foundation

/// What a terminal is doing, as the ambient surfaces count it (`notify::State`).
public enum AmbientState: Sendable, Equatable {
    case idle, working
    /// An agent asks, or a terminal without one rang its bell.
    case needsYou
    /// Finished while nobody was looking, and not seen since.
    case done
    /// A command ended non-zero, or an agent or program stopped on an error, unseen.
    case failed
    case exited
}

/// What tells a terminal's state: an agent, a program that reports its own (OSC 7501), or neither.
public enum AmbientSource: Sendable, Equatable {
    case plain, agent, program
    public var told: Bool { self != .plain }
}

public struct AmbientTerminal: Sendable, Equatable, Identifiable {
    public var session: Int64
    /// The tab that speaks for it, where choosing it goes.
    public var tab: Int64
    public var name: String
    public var state: AmbientState
    /// When it started doing that.
    public var since: Date
    public var source: AmbientSource = .plain
    /// The agent ("Claude Code") or program ("brew") that tells the state, when known.
    public var who: String?
    /// The agent's question, or what it says it is doing.
    public var message: String?
    public var command: String?
    public var took: TimeInterval?
    public var exit: Int?
    public var changed: Int?
    public var folder: String?
    public var branch: String?

    public var id: Int64 { session }

    public init(
        session: Int64, tab: Int64? = nil, name: String, state: AmbientState, since: Date = Date(),
        source: AmbientSource = .plain, who: String? = nil, message: String? = nil, command: String? = nil,
        took: TimeInterval? = nil, exit: Int? = nil, changed: Int? = nil, folder: String? = nil, branch: String? = nil
    ) {
        self.session = session
        self.tab = tab ?? session
        self.name = name
        self.state = state
        self.since = since
        self.source = source
        self.who = who
        self.message = message
        self.command = command
        self.took = took
        self.exit = exit
        self.changed = changed
        self.folder = folder
        self.branch = branch
    }
}
