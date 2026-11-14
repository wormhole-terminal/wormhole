// Rust's "something changed" (`PanelListener.wake`, called on a Rust thread, at most once until the owner drains)
// as the `AsyncStream<Void>` the panel's stores read. The stream coalesces: a flood of file events is one element at
// once and one more per `window` while it lasts; Rust is re-armed with every wake, so nothing after a look goes untold.

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
    /// When the stream last got an element, and whether one is owed for wakes since.
    private var lastYield: DispatchTime?
    private var pending = false

    init(_ continuation: AsyncStream<Void>.Continuation, window: DispatchTimeInterval) {
        self.continuation = continuation
        self.window = window
    }

    /// Rust is re-armed at once, so a change made after this wake is told too; what the stream gets is one element now and,
    /// for the wakes that follow within `window`, one more when it ends. Re-armed only after the window, a change made
    /// between the reader's look and the re-arm was never told: the tab read a file while it was being replaced and kept
    /// what it read (a project's switch put back to its default stayed off in the Project tab).
    func wake() {
        lock.lock()
        let (current, now) = (handle, DispatchTime.now())
        var later: DispatchTime?
        let quiet = lastYield.map { now >= $0 + window } ?? true
        if quiet {
            lastYield = now
        } else if !pending, let last = lastYield {
            pending = true
            later = last + window
        }
        lock.unlock()
        current?.drain()
        if quiet { continuation.yield() }
        if let later { DispatchQueue.global(qos: .utility).asyncAfter(deadline: later) { [weak self] in self?.flush() } }
    }

    /// The window after an element ended and something changed in it: one element for all of it.
    private func flush() {
        lock.lock()
        (pending, lastYield) = (false, .now())
        lock.unlock()
        continuation.yield()
    }

    func attach(_ started: WatchHandle) {
        lock.lock()
        let ended = finished
        if !ended { handle = started }
        lock.unlock()
        // Stopped with the lock let go, as `finish` does: `stop` waits for the watch's own thread to end, and that thread
        // may be inside `wake`, waiting for this lock.
        guard !ended else { return started.stop() }
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
