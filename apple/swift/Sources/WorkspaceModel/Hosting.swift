// Where a terminal's program runs, for the status bar's item and the quit question (the app's `terminal_view::Hosting`).
// "Persistent" terminals run in a session host of their own, which outlives the app; the rest end with it.

import Foundation

public enum HostingState: Sendable, Equatable {
    /// In the app's own process: it ends when the app quits.
    case local
    /// In a session host, which outlives the app.
    case persistent(hostID: String)
    /// Let go of: the program runs on in its host for the next attach (what the app does as it quits).
    case detached(hostID: String)
    /// Its host went away and took the program with it.
    case lost(hostID: String)

    public var hostID: String? {
        switch self {
        case .local: nil
        case .persistent(let id), .detached(let id), .lost(let id): id
        }
    }

    /// Quitting leaves its program running.
    public var survivesQuit: Bool {
        switch self {
        case .persistent, .detached: true
        case .local, .lost: false
        }
    }
}

/// What the quit question counts of the terminals that run something (`QuitFacts`): those in a host that
/// would be left running, and those that would end.
public struct HostedCounts: Sendable, Equatable {
    public var hostedRunning = 0
    public var localRunning = 0
    /// Any terminal at all is in a host, even an idle one: the question's box has something to keep.
    public var anyHosted = false

    public init(hostedRunning: Int = 0, localRunning: Int = 0, anyHosted: Bool = false) {
        (self.hostedRunning, self.localRunning, self.anyHosted) = (hostedRunning, localRunning, anyHosted)
    }

    /// `running` are the terminals that run an agent or a command, `all` every terminal; `hosting` says where each is.
    public init(running: [TerminalID], all: [TerminalID], hosting: (TerminalID) -> HostingState?) {
        anyHosted = all.contains { hosting($0)?.survivesQuit == true }
        for id in running {
            if hosting(id)?.survivesQuit == true { hostedRunning += 1 } else { localRunning += 1 }
        }
    }
}
