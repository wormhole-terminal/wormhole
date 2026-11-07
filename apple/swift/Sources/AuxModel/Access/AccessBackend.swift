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

@MainActor
public protocol AccessBackend: AnyObject {
    /// The question to put on screen now (the first not shown, and only when none is); `nil` when none waits.
    func next() -> AccessQuestion?
    /// Fires when a question was queued or settled, so the driver looks again.
    func changes() -> AsyncStream<Void>
    /// Settles a question with the button pressed (`.deny`, `.session`, `.quit` or `.always`).
    func answer(_ id: UInt64, _ button: AccessButton)
    /// The question went away without an answer (its window closed): it counts as no for now, not remembered.
    func withdraw(_ id: UInt64)
}
