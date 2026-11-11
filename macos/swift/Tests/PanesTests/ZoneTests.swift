import Testing

import PanesModel
import WorkspaceModel

@Suite struct ZoneTests {
    let pane = PaneRect(100, 50, 400, 300)

    @Test func theEdgeBandIsAQuarterOfTheSideUpToACap() {
        #expect(DropBand.depth(400) == 96 && DropBand.depth(300) == 75)
        #expect(DropBand.depth(160) == 40 && DropBand.depth(96) == 24)
        #expect(DropBand.depth(4000) == DropBand.maxDepth && DropBand.depth(0) == 0)
    }

    @Test func aPointNearAnEdgeLandsOnItAndTheMiddleOnTheNearest() {
        #expect(DropBand.zone(in: pane, at: (110, 200)) == .left)
        #expect(DropBand.zone(in: pane, at: (490, 200)) == .right)
        #expect(DropBand.zone(in: pane, at: (300, 60)) == .top)
        #expect(DropBand.zone(in: pane, at: (300, 340)) == .bottom)
        #expect(DropBand.zone(in: pane, at: (220, 200)) == .left)
        #expect(DropBand.zone(in: pane, at: (110, 90)) == .left && DropBand.zone(in: pane, at: (140, 60)) == .top)
    }

    @Test func aTieGoesRightAndANarrowPaneIsNotBiased() {
        let square = PaneRect(0, 0, 200, 200)
        #expect(DropBand.zone(in: square, at: (100, 100)) == .right)
        #expect(DropBand.zone(in: square, at: (100, 120)) == .bottom)
        #expect(DropBand.zone(in: PaneRect(0, 0, 200, 900), at: (60, 450)) == .left)
        #expect(DropBand.zone(in: PaneRect(0, 0, 900, 200), at: (450, 40)) == .top)
    }

    @Test func thePreviewIsTheHalfAndAHintIsItsBandClearOfTheLitHalf() {
        #expect(DropBand.preview(pane, .right) == PaneRect(300, 50, 200, 300))
        #expect(DropBand.preview(pane, .top) == PaneRect(100, 50, 400, 150))
        #expect(DropBand.hint(pane, .left, active: .bottom) == PaneRect(100, 125, 96, 75))
        #expect(DropBand.hint(pane, .top, active: .right) == PaneRect(100, 50, 200, 75))
        #expect(DropBand.hint(PaneRect(0, 0, 0, 0), .top, active: .left) == nil)
    }

    @Test func nothingOutsideAPaneLandsAndASharedEdgeBelongsToOnePane() {
        #expect(DropBand.zone(in: pane, at: (99, 200)) == nil)
        #expect(DropBand.zone(in: pane, at: (500, 200)) == nil)
        #expect(DropBand.zone(in: pane, at: (300, 350)) == nil)
        #expect(DropBand.zone(in: pane, at: (100, 50)) != nil)
        #expect(DropBand.zone(in: pane, at: (.nan, 100)) == nil)
        let rects = [(1, PaneRect(0, 0, 400, 600)), (2, PaneRect(400, 0, 400, 300)), (3, PaneRect(400, 300, 400, 300))]
        let target = { DropBand.target(rects.map { (id: $0.0, rect: $0.1) }, at: $0) }
        #expect(target((399, 300))?.pane == 1 && target((400, 300))?.pane == 3)
        #expect(target((900, 300)) == nil)
    }
}
