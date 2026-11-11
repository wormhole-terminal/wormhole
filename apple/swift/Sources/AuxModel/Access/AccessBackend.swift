// What the approval sheet is driven by (`agent_access/ask.rs`): the questions programs raise through the control
// API, one on screen at a time, and the person's answer to each. The queue and what an answer remembers are
// core's (`ipc::access`); the window only shows a question and says which button was pressed.

import Foundation

/// A question waiting for the person, with the id its answer settles.
public struct AccessQuestion: Equatable, Sendable, Identifiable {
    public var id: UInt64
    public var request: AccessRequest

    public init(id: UInt64, request: AccessRequest) { (self.id, self.request) = (id, request) }
}

/// The question about a mode that was widened outside the app (`config.json` edited by something else): the words of the mode
/// that stays in force and of the one asked for. `token` tells it from an earlier one.
public struct ModeQuestion: Equatable, Sendable {
    public var token: UInt64
    public var previous: String
    public var requested: String

    public init(token: UInt64, previous: String, requested: String) { (self.token, self.previous, self.requested) = (token, previous, requested) }
}

@MainActor
public protocol AccessBackend: AnyObject {
    /// The question about a mode widened outside the app, when one waits to be put on screen (it goes before the programs').
    func nextModeQuestion() -> ModeQuestion?
    /// Whether that question is still waiting: false once it was answered or went unanswered for a minute.
    func isWaitingMode(_ token: UInt64) -> Bool
    /// Keep the widened mode, or put the earlier one back (not answering is not keeping).
    func answerMode(_ token: UInt64, keep: Bool)
    /// Whether question `id` still waits: false once it was settled, withdrawn or went unanswered for a minute.
    func isWaiting(_ id: UInt64) -> Bool
    /// The question to put on screen now (the first not shown, and only when none is); `nil` when none waits.
    func next() -> AccessQuestion?
    /// Fires when a question was queued or settled, so the driver looks again.
    func changes() -> AsyncStream<Void>
    /// Settles a question with the button pressed (`.deny`, `.session`, `.quit` or `.always`).
    func answer(_ id: UInt64, _ button: AccessButton)
    /// The question went away without an answer (its window closed): it counts as no for now, not remembered.
    func withdraw(_ id: UInt64)
}
