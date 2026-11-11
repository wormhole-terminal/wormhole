import CoreGraphics
import Testing

@testable import TerminalUI

@MainActor @Suite struct MinimapTests {
    /// 41 lines (37 of history, a screen of 4), room for 10, live: the numbers `wormhole-term`'s test checks.
    private let live = MinimapGeometry(total: 41, screen: 4, fits: 10, viewTop: 37)

    @Test func theShownLinesFollowTheViewThroughALongHistory() {
        #expect(live.first == 31)
        #expect(MinimapGeometry(total: 41, screen: 4, fits: 10, viewTop: 0).first == 0)
    }

    @Test func theSliderMarksTheScreenInPoints() {
        #expect(live.slider.top == CGFloat(37 - 31) * MinimapGeometry.linePitch)
        #expect(live.slider.height == 4 * MinimapGeometry.linePitch)
    }

    @Test func aLineIsFoundAtItsHeightAndNothingPastTheEnd() {
        #expect(live.line(atY: 0) == 31)
        #expect(live.line(atY: 3) == 32)
        #expect(live.line(atY: 100) == nil)
    }

    @Test func clickingBringsALineToTheMiddleOfTheScreen() {
        #expect(live.viewTop(centering: 20) == 18)
        #expect(live.viewTop(centering: 1) == 0)
        #expect(live.viewTop(centering: 40) == 37, "never past the history")
    }

    @Test func draggingTheSliderToTheEndsReachesTheEndsOfTheHistory() {
        #expect(live.viewTop(forSliderY: 0) == 0)
        #expect(live.viewTop(forSliderY: 1000) == 37)
        #expect(live.displayOffset(viewTop: 37) == 0)
        #expect(live.displayOffset(viewTop: 0) == 37)
    }

    @Test func theMockReadsAsASnapshotThatScrolls() {
        let source = MockMinimap(history: 100, displayOffset: 0)
        let snapshot = source.snapshot(fits: 40)
        #expect(snapshot.lines.count == 40)
        #expect(snapshot.geometry.viewTop == 100)
        source.scroll(toDisplayOffset: 50)
        #expect(source.snapshot(fits: 40).geometry.viewTop == 50)
    }
}
