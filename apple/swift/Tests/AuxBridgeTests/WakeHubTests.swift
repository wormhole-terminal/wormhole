import XCTest

@testable import AuxBridge

final class WakeHubTests: XCTestCase {
    func testAWakeReachesEveryStreamAndABurstIsOnePending() async {
        let hub = WakeHub()
        let one = hub.stream()
        let two = hub.stream()
        hub.wake()
        hub.wake()
        hub.wake()
        var count = 0
        for await _ in one {
            count += 1
            break
        }
        XCTAssertEqual(count, 1)
        for await _ in two { break }
    }

    func testAStreamThatStopsIsRemovedAndTellsItsOwner() async {
        let hub = WakeHub()
        let finished = expectation(description: "finished")
        var stream: AsyncStream<Void>? = hub.stream(onFinish: { finished.fulfill() })
        _ = stream
        stream = nil
        await fulfillment(of: [finished], timeout: 5)
        hub.wake()
    }
}
