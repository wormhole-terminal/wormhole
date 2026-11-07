// The wake of a terminal whose view does not exist yet. The workspace starts a program when the core says a terminal was
// opened; the view that draws it comes later and brings its own wake (`FrameSource.start(size:wake:)`). Rust keeps this
// listener for the terminal's life, so it holds the view's wake once it is given and remembers a wake that came before.

import Synchronization
import WormholeBindings

public final class TerminalRelay: TerminalListener {
    private struct State {
        var target: (@Sendable () -> Void)?
        var missed = false
    }
    private let state = Mutex(State())

    public init() {}

    /// Called by Rust on the terminal's event thread, at most once between two drains.
    public func wake() {
        let target = state.withLock { state -> (@Sendable () -> Void)? in
            if state.target == nil { state.missed = true }
            return state.target
        }
        target?()
    }

    /// The view's wake, from now on; once at once when output came before the view did.
    public func attach(_ wake: @escaping @Sendable () -> Void) {
        let missed = state.withLock { state -> Bool in
            state.target = wake
            defer { state.missed = false }
            return state.missed
        }
        if missed { wake() }
    }
}
