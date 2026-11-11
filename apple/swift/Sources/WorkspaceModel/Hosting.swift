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
    /// A host is being started or attached to: the pane is blank meanwhile.
    case connecting
    /// Its host runs and this app could not attach to it (`HostProblem.unavailable`).
    case unavailable(hostID: String)

    public var hostID: String? {
        switch self {
        case .persistent(let id), .detached(let id), .lost(let id), .unavailable(let id): id
        case .local, .connecting: nil
        }
    }

    /// Quitting leaves its program running.
    public var survivesQuit: Bool {
        switch self {
        case .persistent, .detached, .connecting, .unavailable: true
        case .local, .lost: false
        }
    }
}

/// What is wrong with a terminal's session host, as the bar over its pane says (`terminal_view/host_problem.rs`).
public enum HostProblem: Sendable, Equatable {
    /// The host runs and this app could not attach to it.
    case unavailable(Why)
    /// It stopped answering.
    case notResponding
    /// It went away and took the program.
    case lost

    /// Why a host that runs cannot be shown (core's `UnavailableKind`).
    public enum Why: Sendable, Equatable, CaseIterable {
        case refused, tooOld, tooNew, busy, timedOut, unreachable, failed

        /// May pass by itself, so "Try again" is offered.
        public var isTransient: Bool { [.busy, .timedOut, .unreachable, .failed].contains(self) }
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
