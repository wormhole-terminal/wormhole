// Hears `SettingsListener.wake()`, which Rust calls on a thread of its own with no lock held, and hands the news to
// the main actor: the backend that drains. Holds the backend weakly, so the window closing ends it.

import Foundation
import WormholeBindings

final class WakeRelay: SettingsListener, @unchecked Sendable {
    private let lock = NSLock()
    private weak var backend: RustSettingsBackend?

    func attach(_ backend: RustSettingsBackend) {
        lock.withLock { self.backend = backend }
    }

    func wake() {
        let target = lock.withLock { backend }
        DispatchQueue.main.async { MainActor.assumeIsolated { target?.woke() } }
    }
}
