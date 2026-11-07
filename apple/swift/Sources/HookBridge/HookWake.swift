// Rust calls `wake()` from a thread of its own, at most once between two drains; this turns it into a stream the
// main actor reads, and reading re-arms the next wake. Nothing polls.

import Synchronization
import WormholeBindings

final class HookWake: HookListener, @unchecked Sendable {
    private let continuation = Mutex<AsyncStream<Void>.Continuation?>(nil)
    let stream: AsyncStream<Void>

    init() {
        var made: AsyncStream<Void>.Continuation?
        stream = AsyncStream(bufferingPolicy: .bufferingNewest(1)) { made = $0 }
        continuation.withLock { $0 = made }
    }

    /// Called from a Rust thread with no Rust lock held: only yields.
    func wake() { continuation.withLock { _ = $0?.yield() } }

    func finish() { continuation.withLock { $0?.finish() } }
}
