// The workspace's wake. Rust calls it from a thread of its own, with no lock held, at most once between two drains; it must
// only note that and hop to the main queue (`FFI-CONTRACT.md`, "Threads and locks"). The target is set once the backend
// exists, because the workspace needs its listener to be made.

import Dispatch
import Synchronization
import WormholeBindings

final class WakeRelay: WorkspaceListener {
    private let target = Mutex<(@Sendable () -> Void)?>(nil)

    /// `handler` runs on the main queue each time Rust woke.
    func attach(_ handler: @escaping @MainActor @Sendable () -> Void) {
        target.withLock { $0 = { DispatchQueue.main.async { MainActor.assumeIsolated(handler) } } }
    }

    func wake() { target.withLock { $0 }?() }
}
