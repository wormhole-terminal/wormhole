import CoreGraphics
import Foundation
import Testing

@testable import TerminalUI

@MainActor @Suite struct ScalingTests {
    private let t0 = Date(timeIntervalSinceReferenceDate: 500)
    private func after(_ ms: Int) -> Date { t0.addingTimeInterval(Double(ms) / 1000) }

    @Test func zoomStaysWithinWhatSettingsAllow() {
        #expect(TextZoom.zoomed(base: 14, zoom: 0, step: 1) == 1)
        #expect(TextZoom.zoomed(base: 14, zoom: 3, step: -1) == 2)
        #expect(TextZoom.zoomed(base: 35, zoom: 1, step: 1) == 1, "36 at most")
        #expect(TextZoom.zoomed(base: 9, zoom: -1, step: -1) == -1, "8 at least")
    }

    @Test func theEaseStartsWhereItWasAndEndsExactlyOnTheTarget() {
        let ease = TextZoomEase(from: 0, to: 1, began: t0)
        #expect(ease.at(t0) == .init(size: 0, done: false))
        #expect(ease.at(after(120)) == .init(size: 1, done: true))
        #expect(TextZoomEase(from: 0, to: 0.3, began: t0).at(after(120)).size == 0.3, "the end is not rounded")
    }

    @Test func theSizesOnTheWayAreQuartersOfAPointAndOnlyGrow() {
        let ease = TextZoomEase(from: 2, to: 5, began: t0)
        var before = 2.0
        for ms in 0..<120 {
            let sample = ease.at(after(ms))
            #expect(sample.size == TextZoomEase.quantise(sample.size))
            #expect(sample.size >= before && sample.size <= 5)
            before = sample.size
        }
    }

    @Test func aSecondPressGoesOnFromTheSizeTheFirstReached() {
        let ease = TextZoomEase(from: 0, to: 4, began: t0)
        let mid = ease.at(after(60)).size
        let again = ease.retargeted(at: after(60), to: 8)
        #expect(abs(again.at(after(60)).size - mid) <= TextZoomEase.quantum)
        #expect(again.at(after(60 + 120)).size == 8)
    }

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

    @Test func theWindowStepsByAWholeCellWidthAndLeavesAFractionalOneFree() {
        #expect(ResizeSteps.increments(cell: CGSize(width: 8, height: 17.6)) == CGSize(width: 8, height: 18))
        #expect(ResizeSteps.increments(cell: CGSize(width: 8.4, height: 17)) == CGSize(width: 1, height: 17))
    }

    @Test func settlingTakesOffTheStripAndNeverAWholeCell() {
        let cell = CGSize(width: 8, height: 17)
        #expect(ResizeSteps.settle(spare: CGSize(width: 3, height: 0.5), cell: cell) == CGSize(width: 3, height: 0))
        #expect(ResizeSteps.settle(spare: CGSize(width: 0.5, height: 0.9), cell: cell) == nil)
        #expect(ResizeSteps.settle(spare: CGSize(width: 8, height: 17), cell: cell) == nil)
    }
}
