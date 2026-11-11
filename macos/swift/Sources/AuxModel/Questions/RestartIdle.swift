// When "Restart When Idle" may go ahead (`update::restart`): every terminal idle, without a break, for a while. An agent is idle
// for a moment between its steps, so a single quiet look would restart under it. No clock of its own: each look says what time it is.

import Foundation

public enum RestartIdle {
    /// How long every terminal must stay idle before the app restarts by itself (`IDLE_FOR`).
    public static let idleFor: TimeInterval = 30

    /// A terminal that lets the restart go ahead: no agent working or waiting for an answer, and no command of its own in
    /// front. An agent sitting at its prompt has its program in front too, but it is waiting for you, not working.
    public static func isIdle(_ status: RunningTerminal.Status, ended: Bool, commandRunning: Bool, agentInFront: Bool) -> Bool {
        if ended { return true }
        return switch status {
        case .busy, .attention: false
        case .idle: !commandRunning || agentInFront
        }
    }
}

/// How long the terminals have been idle, as seen from one look after another.
public struct IdleWatch: Sendable {
    private var since: Date?

    public init() {}

    /// One look at the terminals: `true` once every look for `idleFor` found them idle; a busy one starts the wait over.
    public mutating func seen(idle: Bool, at now: Date) -> Bool {
        guard idle else {
            since = nil
            return false
        }
        let start = since ?? now
        since = start
        return now.timeIntervalSince(start) >= RestartIdle.idleFor
    }
}
