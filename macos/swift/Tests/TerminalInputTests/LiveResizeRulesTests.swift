#if WITH_UNIFFI
import CoreGraphics
import Testing
import UniFFISource

@testable import TerminalCore

/// The live-resize rules are the core's: Swift asks, the answers are what `wormhole_core::view::motion::live_resize` gives.
@Suite(.serialized) struct LiveResizeRulesTests {
    @Test func theCoresRulesAnswerTheWindow() {
        installResizeRules()
        let cell = CGSize(width: 8.5, height: 18), spare = CGSize(width: 3, height: 5)
        let current = CGSize(width: 1000, height: 640), least = CGSize(width: 640, height: 360)
        // Dragged 30 pt wider and 50 pt taller: whole cells from where the grid ends.
        let snapped = ResizeSteps.snapped(CGSize(width: 1030, height: 690), from: current, spare: spare, cell: cell, least: least)
        #expect(snapped == CGSize(width: 1000 + 4 * 8.5 - 3, height: 640 + 3 * 18 - 5))
        #expect(ResizeSteps.snapped(current, from: current, spare: .zero, cell: cell, least: least) == current)

        #expect(ResizeSteps.settle(spare: CGSize(width: 3, height: 0.5), cell: CGSize(width: 8, height: 17)) == CGSize(width: 3, height: 0))
        #expect(ResizeSteps.settle(spare: CGSize(width: 8, height: 17), cell: CGSize(width: 8, height: 17)) == nil, "a whole cell is the person's size")

        let screen = CGRect(x: 0, y: 0, width: 1600, height: 1000)
        let window = CGRect(x: 200, y: 100, width: 800, height: 600)
        #expect(ResizeSteps.settledFrame(window, strip: CGSize(width: 3, height: 5), visible: screen, heldLeft: true, heldTop: true)
            == CGRect(x: 203, y: 100, width: 797, height: 595), "the right and bottom edges stay")
        #expect(ResizeSteps.settledFrame(CGRect(x: 0, y: 0, width: 800, height: 990), strip: CGSize(width: 3, height: 5), visible: screen, heldLeft: false, heldTop: false) == nil, "a tiled window stays")
    }

    @Test func aGateTellsTheProgramAtMostEvery100msAndAtOnceWhenTheDragEnds() {
        installResizeRules()
        let gate = ResizeGate()
        #expect(gate.step(live: true, differs: true).apply, "the first size of a drag is not made to wait")
        let held = gate.step(live: true, differs: true)
        #expect(!held.apply)
        #expect((held.wakeIn ?? 1) <= 0.1)
        #expect(gate.step(live: false, differs: true).apply, "the drag ended: told at once")
        #expect(!gate.step(live: false, differs: false).apply)
    }
}
#endif
