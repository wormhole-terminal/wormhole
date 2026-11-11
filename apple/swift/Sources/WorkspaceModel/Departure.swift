// Terminals on their way from one window to another, programs running (a group's "Move to new window"). What a backend
// needs to hand them over is its own business (the Rust workspace keeps the running terminals and their wakes), so
// the shell only carries this token from `takeOut` to the other window's `receive`.

import Foundation

public final class TerminalDeparture {
    /// The terminals, in the window's order.
    public let ids: [TerminalID]
    /// What the backend that made it needs to take them in again: opaque here.
    public let payload: AnyObject

    public init(ids: [TerminalID], payload: AnyObject) {
        self.ids = ids
        self.payload = payload
    }
}
