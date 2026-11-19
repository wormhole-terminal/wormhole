// How often a terminal that no frame is drawn for is drained: its window is in the Dock, or its view is in no window (a
// terminal that is not the shown one). A view in sight is drained once per frame by its display link; without one, a
// drain on every wake would be a hop to the main queue per read burst of a program that prints fast, which the one-wake
// queue exists to prevent (`crates/wormhole-ffi/src/events.rs`). So the first wake is drained at once (a bell, a title,
// an exit are heard without delay) and, for an interval after any drain, the wakes that follow are one drain at its end.
// The source wakes once until it is drained, so a flood is one hop and one drain per interval.

import Foundation

/// The pace of the drains of a terminal no frame is drawn for: at once for a lone wake, once per interval under a flood.
public struct DrainPace: Sendable {
    /// How long after a drain the next one waits.
    public static let interval: TimeInterval = 0.1

    /// A drain was made and its interval has not ended.
    private var running = false
    /// A wake came during the interval: its end drains.
    private var owed = false

    public init() {}

    /// The source woke. True: drain now, and say `elapsed()` after `interval`. False: the interval's end drains.
    public mutating func woke() -> Bool {
        guard running else {
            running = true
            return true
        }
        owed = true
        return false
    }

    /// The interval ended. True: a wake came meanwhile, so drain now and say `elapsed()` after another interval.
    public mutating func elapsed() -> Bool {
        guard owed else {
            running = false
            return false
        }
        owed = false
        return true
    }
}
