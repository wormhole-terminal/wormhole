// Rust's "something changed" (`PanelListener.wake`, called on a Rust thread, at most once until the owner drains)
// as the `AsyncStream<Void>` the panel's stores read. The stream coalesces: a flood of file events is one element
// per `window`, then Rust is re-armed to wake again.

import Foundation
import WormholeBindings

/// What a started watch gives back: re-arm Rust's wake, and end it.
struct WatchHandle: Sendable {
    let drain: @Sendable () -> Void
    let stop: @Sendable () -> Void
}

/// The listener Rust calls, feeding one stream. It holds the watch only until the stream ends, which breaks the
/// cycle (Rust keeps the listener, the listener keeps the watch) at `finish`.
final class WakeRelay: PanelListener, @unchecked Sendable {
    private let continuation: AsyncStream<Void>.Continuation
    private let window: DispatchTimeInterval
    private let lock = NSLock()
    private var handle: WatchHandle?
    private var finished = false

    init(_ continuation: AsyncStream<Void>.Continuation, window: DispatchTimeInterval) {
        self.continuation = continuation
        self.window = window
    }

    func wake() {
        continuation.yield()
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + window) { [weak self] in self?.rearm() }
    }

    func attach(_ started: WatchHandle) {
        lock.lock()
        defer { lock.unlock() }
        guard !finished else { return started.stop() }
        handle = started
        // A wake before the handle existed could not be re-armed.
        started.drain()
    }

    func finish() {
        lock.lock()
        finished = true
        let ended = handle
        handle = nil
        lock.unlock()
        ended?.stop()
    }

    private func rearm() {
        lock.lock()
        let current = handle
        lock.unlock()
        current?.drain()
    }
}

enum WakeStream {
    /// A stream that yields when the watch `start` makes wakes its listener. `start` runs on a background queue
    /// (opening an event stream and reading a folder are not main-actor work).
    static func make(
        window: DispatchTimeInterval = .milliseconds(250), start: @escaping @Sendable (WakeRelay) -> WatchHandle
    ) -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let relay = WakeRelay(continuation, window: window)
            DispatchQueue.global(qos: .userInitiated).async { relay.attach(start(relay)) }
            continuation.onTermination = { _ in relay.finish() }
        }
    }
}
