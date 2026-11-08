import CoreGraphics
import Foundation
import Testing

@testable import TerminalCore
@testable import TerminalUI

@MainActor @Suite struct ScalingTests {
    private let t0 = Date(timeIntervalSinceReferenceDate: 500)
    private func after(_ ms: Int) -> Date { t0.addingTimeInterval(Double(ms) / 1000) }

    @Test func theChipIsWholeFor700msThenFadesOver150AndReduceMotionCutsIt() {
        #expect(SizeReadoutPhase.of(since: 0, cut: false) == .holding(0.7))
        #expect(SizeReadoutPhase.of(since: 0.7, cut: false) == .fading(1))
        if case let .fading(half) = SizeReadoutPhase.of(since: 0.775, cut: false) { #expect(abs(half - 0.5) < 0.001) } else { Issue.record("fading") }
        #expect(SizeReadoutPhase.of(since: 0.85, cut: false) == .gone)
        #expect(SizeReadoutPhase.of(since: 0.7, cut: true) == .gone)
    }

    @Test func theChipSaysASizeAfterALiveResizeChangedItAndAfterAZoomButNeverOnOpen() {
        let model = SizeReadoutModel()
        model.noteSize(cols: 100, rows: 30, live: true, at: t0)
        #expect(model.text == nil, "the first size says nothing")
        model.noteSize(cols: 100, rows: 30, live: true, at: after(10))
        #expect(model.text == nil, "unchanged")
        model.noteSize(cols: 110, rows: 30, live: false, at: after(20))
        #expect(model.text == nil, "a size change outside a live resize says nothing")
        model.noteSize(cols: 120, rows: 31, live: true, at: after(30))
        #expect(model.text == "120 × 31")
    }

    @Test func aZoomSaysTheSizeOnceItIsSettledUnlessItIsOldOrOffScreen() {
        let model = SizeReadoutModel()
        model.noteSize(cols: 100, rows: 30, live: false, at: t0)
        model.textZoomed(onScreen: true, at: t0)
        model.noteSize(cols: 90, rows: 27, live: false, at: after(500))
        #expect(model.text == "90 × 27")
        let stale = SizeReadoutModel()
        stale.textZoomed(onScreen: true, at: t0)
        stale.noteSize(cols: 90, rows: 27, live: false, at: after(2500))
        #expect(stale.text == nil)
        let away = SizeReadoutModel()
        away.textZoomed(onScreen: false, at: t0)
        away.noteSize(cols: 90, rows: 27, live: false, at: after(10))
        #expect(away.text == nil)
    }

    @Test func theGateTellsTheProgramAtMostEvery100msAndOnceMoreWhenTheDragEnds() {
        var gate = ResizeGate()
        #expect(gate.step(now: t0, live: true, differs: true) == .init(apply: true, wakeIn: nil))
        let held = gate.step(now: after(30), live: true, differs: true)
        #expect(!held.apply)
        #expect(abs((held.wakeIn ?? 0) - 0.07) < 0.001)
        #expect(gate.step(now: after(100), live: true, differs: true).apply)
        #expect(gate.step(now: after(110), live: false, differs: true).apply, "the drag ended: told at once")
        #expect(!gate.step(now: after(120), live: false, differs: false).apply)
    }

    @Test func aSizeToldBeforeTheDragDoesNotMakeItsFirstWait() {
        var gate = ResizeGate()
        #expect(gate.step(now: t0, live: false, differs: true).apply)
        #expect(gate.step(now: after(10), live: true, differs: true).apply)
    }

    @Test func aDragStepsByWholeCellsFromWhereTheGridEndsWhateverTheChromeMeasures() {
        let cell = CGSize(width: 8.5, height: 18), spare = CGSize(width: 3, height: 5)
        let current = CGSize(width: 1000, height: 640), least = CGSize(width: 640, height: 360)
        // Dragged 30 pt wider and 50 pt taller: the nearest whole cells (33 pt of the strip and the drag is 3.9 cells, 55 pt is 3.05).
        let snapped = ResizeSteps.snapped(CGSize(width: 1030, height: 690), from: current, spare: spare, cell: cell, least: least)
        #expect(snapped == CGSize(width: 1000 + 4 * 8.5 - 3, height: 640 + 3 * 18 - 5))
        // The size that results holds whole cells: nothing beyond the grid.
        #expect((spare.width + snapped.width - current.width).truncatingRemainder(dividingBy: cell.width) == 0)
        #expect(ResizeSteps.snapped(current, from: current, spare: .zero, cell: cell, least: least) == current)
        // Never under the least size, passing it by a whole cell.
        let small = ResizeSteps.snapped(CGSize(width: 100, height: 100), from: current, spare: .zero, cell: cell, least: least)
        #expect(small.width >= 640 && small.height >= 360 && small.width < 640 + cell.width && small.height < 360 + cell.height)
    }

    @Test func settlingTakesOffTheStripAndNeverAWholeCell() {
        let cell = CGSize(width: 8, height: 17)
        #expect(ResizeSteps.settle(spare: CGSize(width: 3, height: 0.5), cell: cell) == CGSize(width: 3, height: 0))
        #expect(ResizeSteps.settle(spare: CGSize(width: 0.5, height: 0.9), cell: cell) == nil)
        #expect(ResizeSteps.settle(spare: CGSize(width: 8, height: 17), cell: cell) == nil)
    }

    @Test func theStripComesOffTheSideThePersonDidNotHoldAndTiledWindowsStay() {
        let screen = CGRect(x: 0, y: 0, width: 1600, height: 1000)
        let window = CGRect(x: 200, y: 100, width: 800, height: 600)
        let strip = CGSize(width: 3, height: 5)
        #expect(ResizeSteps.settledFrame(window, strip: strip, visible: screen, heldLeft: false, heldTop: false)
            == CGRect(x: 200, y: 105, width: 797, height: 595), "the top edge stays: the bottom one moves up")
        #expect(ResizeSteps.settledFrame(window, strip: strip, visible: screen, heldLeft: true, heldTop: true)
            == CGRect(x: 203, y: 100, width: 797, height: 595), "the right and bottom edges stay")
        #expect(ResizeSteps.settledFrame(CGRect(x: 0, y: 0, width: 800, height: 990), strip: strip, visible: screen, heldLeft: false, heldTop: false) == nil)
    }
}
