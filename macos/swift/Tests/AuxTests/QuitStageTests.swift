import XCTest
@testable import AuxModel

/// A quit under way absorbs a second request (`AppController+Quit` asks this rule in `applicationShouldTerminate`, in
/// `quit(endAll:)` and before it asks AppKit at all). Order guarded: ⌘Q puts the question; a second ⌘Q, the Dock's Quit
/// or the last window's close arrives before the person answered.
final class QuitStageTests: XCTestCase {
    func testAFirstRequestIsDecided() {
        XCTAssertEqual(QuitStage.reply(during: nil, asks: true), .decide)
        XCTAssertEqual(QuitStage.reply(during: nil, asks: false), .decide)
    }

    func testASecondRequestWhileTheQuestionIsUpIsAbsorbed() {
        XCTAssertEqual(QuitStage.reply(during: .asking, asks: true), .absorbed, "one question, not a second sheet behind it")
    }

    func testAnUnattendedQuitIsNotHeldUpByTheQuestion() {
        XCTAssertEqual(QuitStage.reply(during: .asking, asks: false), .decide, "`kill` and the control folder quit with a question up")
    }

    func testARequestWhileTheHostsEndWaitsForThatEndAndGoesOnceTheyEnded() {
        for asks in [true, false] {
            XCTAssertEqual(QuitStage.reply(during: .endingHosts, asks: asks), .later, "one End All")
            XCTAssertEqual(QuitStage.reply(during: .hostsEnded, asks: asks), .now)
        }
    }
}
