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
        guard case .none = tracker.observe(lone, motion: Always(), plays: true, split: nil) else { return XCTFail("the first layout is no change") }
        guard case .plays(let first) = tracker.observe(two, motion: Always(), plays: true, split: 2) else { return XCTFail("a pane came") }
        guard case .plays(let again) = tracker.observe(two, motion: Always(), plays: true, split: 2) else { return XCTFail("the same layout") }
        XCTAssertEqual(first, again, "drawn twice, one opening")
        XCTAssertEqual(first.new, 2)
        guard case .none = tracker.observe(lone, motion: Always(), plays: true, split: 2) else { return XCTFail("a pane left") }
    }

    func testWithMotionOffTheSplitIsCutAndWithoutTheCoreNothingIsAsked() {
        let tracker = SplitOpeningTracker()
        _ = tracker.observe(lone, motion: Always(), plays: false, split: nil)
        XCTAssertTrue(tracker.observe(two, motion: Always(), plays: false, split: 2).isCut)
        let sample = SplitOpeningTracker()
        _ = sample.observe(lone, motion: nil, plays: true, split: nil)
        guard case .none = sample.observe(two, motion: nil, plays: true, split: 2) else { return XCTFail("no core, no opening") }
    }

    /// GPUI plays the opening only for ⌘D and ⇧⌘D (`split_pane`): a pane that came by a drop on a pane's edge or a join from
    /// the list lands at once, even where it took half a pane's room; and a split's pane that left and comes back is a drop.
    func testOnlyThePaneASplitOpenedPlays() {
        let tracker = SplitOpeningTracker()
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil)
        guard case .none = tracker.observe(two, motion: Always(), plays: true, split: nil) else { return XCTFail("a join is a cut") }
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil)
        guard case .none = tracker.observe(two, motion: Always(), plays: true, split: 7) else { return XCTFail("another split's pane") }
        _ = tracker.observe(lone, motion: Always(), plays: true, split: 2)
        guard case .none = tracker.observe(two, motion: Always(), plays: true, split: 2) else {
            return XCTFail("the split's pane was weighed already: dropped back, it is a drop")
        }
        let fresh = SplitOpeningTracker()
        _ = fresh.observe(lone, motion: Always(), plays: true, split: nil)
        guard case .plays = fresh.observe(two, motion: Always(), plays: true, split: 2) else { return XCTFail("the split's own pane plays") }
    }

    func testAPaneZoomedIsNoOpening() {
        var zoomed = two
        zoomed.zoomed = 2
        let tracker = SplitOpeningTracker()
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil)
        guard case .none = tracker.observe(zoomed, motion: Always(), plays: true, split: 2) else { return XCTFail("a zoom is a cut") }
    }

    func testALayoutThatOnlyFollowsTheAreasSizeLandsAtOnce() {
        let tracker = SplitOpeningTracker()
        let area = CGSize(width: 100, height: 50)
        let bigger = PanesState(shown: [PanePlace(1, PaneRect(0, 0, 300, 120))], focused: 1)
        // Before the area is known the core's panes are empty, then they fill it: the first layout is a cut, not a growth.
        _ = tracker.observe(PanesState(shown: [PanePlace(1, PaneRect(0, 0, 0, 0))], focused: 1), motion: Always(), plays: true, split: nil, area: area)
        XCTAssertTrue(tracker.observe(lone, motion: Always(), plays: true, split: nil, area: area).isCut, "the first fill")
        // A window resize: the same panes for a new size.
        XCTAssertTrue(tracker.observe(bigger, motion: Always(), plays: true, split: nil, area: CGSize(width: 300, height: 120)).isCut, "a new size")
        // A split in an area the panes fill is still an opening.
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil, area: area)
        guard case .plays = tracker.observe(two, motion: Always(), plays: true, split: 2, area: area) else { return XCTFail("a split of the area's panes plays") }
    }

    func testALayoutThatLandsAfterTheAreaMovedTwiceIsCutWhicheverSizeItWasLaidFor() {
        let tracker = SplitOpeningTracker()
        let small = CGSize(width: 100, height: 50)
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil, area: small)
        // The window grew: the area was 300 wide for a body, then 260 once the panel took its share; the core answered for 300.
        let for300 = PanesState(shown: [PanePlace(1, PaneRect(0, 0, 300, 120))], focused: 1)
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil, area: CGSize(width: 300, height: 120))
        XCTAssertTrue(tracker.observe(for300, motion: Always(), plays: true, split: nil, area: CGSize(width: 260, height: 120)).isCut, "laid for the size before")
        // The next split is its own opening: the area has not moved since.
        let back = PanesState(shown: [PanePlace(1, PaneRect(0, 0, 100, 50))], focused: 1)
        _ = tracker.observe(back, motion: Always(), plays: true, split: nil, area: small)
        _ = tracker.observe(lone, motion: Always(), plays: true, split: nil, area: small)
        guard case .plays = tracker.observe(two, motion: Always(), plays: true, split: 2, area: small) else { return XCTFail("a split after the area settled plays") }
    }
}
