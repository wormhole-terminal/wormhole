// The Rust objects tell Swift that something changed through one coalesced `wake()` on a thread of their own
// (`AuxListener`); this turns it into streams the main actor reads, and re-reads the object's state, which re-arms
// the next wake. Nothing here polls.

import Foundation
import Synchronization
import WormholeBindings

final class WakeHub: AuxListener, @unchecked Sendable {
    private let continuations = Mutex<[UUID: AsyncStream<Void>.Continuation]>([:])

    /// Called from a Rust thread with no Rust lock held: only yields.
    func wake() {
        let all = continuations.withLock { Array($0.values) }
        for continuation in all { continuation.yield() }
    }

    /// A stream that yields once per wake (one pending at most); ends when its consumer stops.
    func stream(onFinish: (@Sendable () -> Void)? = nil) -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let key = UUID()
            continuations.withLock { $0[key] = continuation }
            continuation.onTermination = { [weak self] _ in
                self?.continuations.withLock { _ = $0.removeValue(forKey: key) }
                onFinish?()
            }
        }
    }
}
