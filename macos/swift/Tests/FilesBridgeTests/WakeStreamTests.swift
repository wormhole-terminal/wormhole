import XCTest

@testable import FilesBridge

/// Rust's wake as the stream the stores read: coalesced, re-armed, and ended with the stream.
final class WakeStreamTests: XCTestCase {
    private final class Counter: @unchecked Sendable {
        private let lock = NSLock()
        private var value = [String: Int]()
        func bump(_ key: String) { lock.withLock { value[key, default: 0] += 1 } }
        subscript(key: String) -> Int { lock.withLock { value[key, default: 0] } }
    }

    func testAFloodIsOneElementThenTheWatchIsRearmed() async throws {
        let counter = Counter()
        let held = LockedRelay()
        let stream = WakeStream.make(window: .milliseconds(50)) { relay in
            held.set(relay)
            return WatchHandle(drain: { counter.bump("drain") }, stop: { counter.bump("stop") })
        }
        var iterator = stream.makeAsyncIterator()
        let relay = try await held.waitForRelay()
        for _ in 0..<200 { relay.wake() }
        _ = await iterator.next()
        try await Task.sleep(for: .milliseconds(200))
        XCTAssertGreaterThanOrEqual(counter["drain"], 2, "armed once at the start, then after the window")
        XCTAssertEqual(counter["stop"], 0)
    }

    /// A change right after the reader looked (inside the window) is told when the window ends, not lost: the reader may
    /// have read the file while it was being replaced.
    func testAWakeInsideTheWindowIsAnElementOfItsOwn() async throws {
        let held = LockedRelay()
        let stream = WakeStream.make(window: .milliseconds(100)) { relay in
            held.set(relay)
            return WatchHandle(drain: {}, stop: {})
        }
        var iterator = stream.makeAsyncIterator()
        let relay = try await held.waitForRelay()
        relay.wake()
        _ = await iterator.next()
        relay.wake()
        let second = Task { () -> Bool in
            var iterator = iterator
            return await iterator.next() != nil
        }
        // A deadline, so a wake that was lost fails the test instead of holding it.
        let deadline = DispatchWorkItem { second.cancel() }
        DispatchQueue.global().asyncAfter(deadline: .now() + 3, execute: deadline)
        let told = await second.value
        deadline.cancel()
        XCTAssertTrue(told, "the second wake reaches the reader")
    }

    func testEndingTheStreamStopsTheWatchEvenIfItStartedLate() async throws {
        let counter = Counter()
        let stream = WakeStream.make { _ in
            Thread.sleep(forTimeInterval: 0.1)
            return WatchHandle(drain: {}, stop: { counter.bump("stop") })
        }
        let reader = Task { for await _ in stream {} }
        reader.cancel()
        try await Task.sleep(for: .milliseconds(400))
        XCTAssertEqual(counter["stop"], 1, "the reader went away before the watch existed: it is stopped as it arrives")
    }

    /// Order: the stream ended before its watch was made; the watch arrives and is stopped, which waits for the watch's
    /// own thread to end (`FolderWatch.stop` joins it); that thread tells of a change at that instant. With the relay's
    /// lock held across `stop`, the thread waits for the lock and `stop` for the thread, for good.
    func testAWatchThatArrivesAfterTheEndIsStoppedWithTheLockLetGo() {
        let (_, continuation) = AsyncStream<Void>.makeStream()
        let relay = WakeRelay(continuation, window: .milliseconds(50))
        relay.finish()
        let counter = Counter()
        let late = WatchHandle(
            drain: {},
            stop: {
                let through = DispatchSemaphore(value: 0)
                DispatchQueue.global().async {
                    relay.wake()
                    through.signal()
                }
                // A stand-in for the join: it gives up after a while, where the real one would not.
                if through.wait(timeout: .now() + 5) == .success { counter.bump("through") }
            })
        relay.attach(late)
        XCTAssertEqual(counter["through"], 1, "the watch's thread got through `wake` while `stop` waited for it")
    }
}

private final class LockedRelay: @unchecked Sendable {
    private let lock = NSLock()
    private var relay: WakeRelay?

    func set(_ relay: WakeRelay) { lock.withLock { self.relay = relay } }

    func waitForRelay() async throws -> WakeRelay {
        for _ in 0..<100 {
            if let found = lock.withLock({ relay }) { return found }
            try await Task.sleep(for: .milliseconds(20))
        }
        throw CocoaError(.fileReadUnknown)
    }
}
