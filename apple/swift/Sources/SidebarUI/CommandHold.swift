// The ⌘1–9 hints: keycaps that show only while ⌘ is held on its own for a moment (core's `CmdHold`). A quick ⌘C or ⌘T
// never flashes them; they go on release, on any other key, and when the window loses focus. Time is passed in.

import Foundation

struct CommandHold: Equatable, Sendable {
    /// How long ⌘ has to be held alone before the keycaps show (`HOLD_DELAY`).
    static let delay: TimeInterval = 0.25

    private var since: Date?
    private var spoiled = false

    /// The modifier keys changed; `commandAlone` is whether ⌘ is now the only one held.
    mutating func modifiers(commandAlone: Bool, at now: Date) {
        if !commandAlone { self = CommandHold() } else if since == nil { self = CommandHold(since: now, spoiled: false) }
    }

    /// A key other than a modifier went down.
    mutating func key() { if since != nil { spoiled = true } }

    /// The window lost focus: whatever is held, it is not for this window any more.
    mutating func blur() { self = CommandHold() }

    func showing(at now: Date) -> Bool { !spoiled && since.map { now.timeIntervalSince($0) >= Self.delay } == true }

    /// While ⌘ is held and the hints are still to come, how long until they show.
    func due(at now: Date) -> TimeInterval? {
        guard let since, !spoiled else { return nil }
        let held = now.timeIntervalSince(since)
        return held < Self.delay ? Self.delay - held : nil
    }
}
