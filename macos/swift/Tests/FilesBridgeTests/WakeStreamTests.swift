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
        let timeout = Task {
            try await Task.sleep(for: .seconds(3))
            second.cancel()
        }
        let told = await second.value
        timeout.cancel()
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
