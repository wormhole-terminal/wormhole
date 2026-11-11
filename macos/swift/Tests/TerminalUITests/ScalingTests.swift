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
}
