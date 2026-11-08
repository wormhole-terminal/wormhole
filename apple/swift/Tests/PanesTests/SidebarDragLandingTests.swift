import CoreGraphics
import PanesModel
import WorkspaceModel
import XCTest

@testable import PanesUI

/// The pointer of a drag aims at the pane and edge under it; what the drop then does is the core's.
final class SidebarDragLandingTests: XCTestCase {
    func testThePointerAimsAtThePaneAndEdgeUnderIt() {
        // 1 on the left, 2 above 3 on the right, in an area of 900 x 600.
        let panes = PanesState(
            shown: [
                PanePlace(1, PaneRect(0, 0, 450, 600)), PanePlace(2, PaneRect(450, 0, 450, 300)),
                PanePlace(3, PaneRect(450, 300, 450, 300)),
            ], focused: 2)
        var state = PaneDropState(dragged: DraggedTerminal(id: 99, origin: .sidebarRow))
        state.aim(at: CGPoint(x: 896, y: 450), in: panes)
        XCTAssertEqual(state.landing?.target, 3)
        XCTAssertEqual(state.landing?.zone, .right, "near the right edge of the pane")
        state.aim(at: CGPoint(x: -20, y: 300), in: panes)
        XCTAssertNil(state.landing, "left of the area is no pane")
    }

    func testADraggedTerminalTravelsAsText() {
        let dragged = DraggedTerminal(id: 7, origin: .paneHeader)
        XCTAssertEqual(DraggedTerminal(encoded: dragged.encoded), dragged)
        XCTAssertNil(DraggedTerminal(encoded: "hello"))
        XCTAssertNil(DraggedTerminal(encoded: "wormhole.terminal:x:paneHeader"))
    }
}
