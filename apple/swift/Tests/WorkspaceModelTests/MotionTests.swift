import XCTest

@testable import WorkspaceModel

final class MotionTests: XCTestCase {
    func testMovingDurationsStayWithinTheShortRange() {
        for duration in [Motion.feedback, Motion.standard, Motion.slide] {
            XCTAssertTrue((0.15...0.35).contains(duration))
        }
        XCTAssertLessThan(Motion.feedback, Motion.standard)
        XCTAssertLessThan(Motion.standard, Motion.slide)
    }

    func testReducedMotionFadeIsShorterThanAnyMovement() {
        XCTAssertLessThan(Motion.reducedFade, Motion.feedback)
    }
}
