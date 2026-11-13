import PanesModel
import WorkspaceModel
import XCTest

@testable import PanesUI

/// Which layout changes the area plays as an opening: the core's rule says whether one is, the tracker decides once per layout
/// and keeps it while that layout stands.
@MainActor
final class SplitOpeningTrackerTests: XCTestCase {
    /// A stand-in for the core: a pane that is new is always an opening.
    private struct Always: SplitMotion {
        func opening(before: [PanePlace], after: [PanePlace], new: PaneID) -> SplitOpening? {
            SplitOpening(target: 1, row: true, whole: PaneRect(0, 0, 100, 50), newPane: PaneRect(50, 0, 50, 50), duration: 0.4)
        }

        func frame(of opening: SplitOpening, elapsed: Double) -> SplitFrame {
            SplitFrame(kept: opening.whole, newPane: opening.newPane)
        }
    }

    private let lone = PanesState(shown: [PanePlace(1, PaneRect(0, 0, 100, 50))], focused: 1)
    private let two = PanesState(shown: [PanePlace(1, PaneRect(0, 0, 50, 50)), PanePlace(2, PaneRect(50, 0, 50, 50))], focused: 2)

    func testASplitThatCameIsPlayedAndStandsUntilTheLayoutChanges() {
        let tracker = SplitOpeningTracker()
        guard case .none = tracker.observe(lone, motion: Always(), plays: true) else { return XCTFail("the first layout is no change") }
        guard case .plays(let first) = tracker.observe(two, motion: Always(), plays: true) else { return XCTFail("a pane came") }
        guard case .plays(let again) = tracker.observe(two, motion: Always(), plays: true) else { return XCTFail("the same layout") }
        XCTAssertEqual(first, again, "drawn twice, one opening")
        XCTAssertEqual(first.new, 2)
        guard case .none = tracker.observe(lone, motion: Always(), plays: true) else { return XCTFail("a pane left") }
    }

    func testWithMotionOffTheSplitIsCutAndWithoutTheCoreNothingIsAsked() {
        let tracker = SplitOpeningTracker()
        _ = tracker.observe(lone, motion: Always(), plays: false)
        XCTAssertTrue(tracker.observe(two, motion: Always(), plays: false).isCut)
        let sample = SplitOpeningTracker()
        _ = sample.observe(lone, motion: nil, plays: true)
        guard case .none = sample.observe(two, motion: nil, plays: true) else { return XCTFail("no core, no opening") }
    }

    func testAPaneZoomedIsNoOpening() {
        var zoomed = two
        zoomed.zoomed = 2
        let tracker = SplitOpeningTracker()
        _ = tracker.observe(lone, motion: Always(), plays: true)
        guard case .none = tracker.observe(zoomed, motion: Always(), plays: true) else { return XCTFail("a zoom is a cut") }
    }
}
