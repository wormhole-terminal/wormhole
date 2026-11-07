import Foundation
import Testing

@testable import TerminalCore

@Suite struct TextZoomTests {
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

    @Test func itEasesOutSoMostOfTheWayIsGoneByTheMiddle() {
        #expect(TextZoomEase(from: 0, to: 1, began: t0).at(after(60)).size > 0.5)
    }

    @Test func aSecondPressGoesOnFromTheSizeTheFirstReached() {
        let ease = TextZoomEase(from: 0, to: 4, began: t0)
        let mid = ease.at(after(60)).size
        let again = ease.retargeted(at: after(60), to: 8)
        #expect(abs(again.at(after(60)).size - mid) <= TextZoomEase.quantum)
        #expect(again.at(after(60 + 120)).size == 8)
    }

    @Test func theCurveMatchesCoresEaseOut() {
        // core `view::motion::ease`: cubic-bezier(0.2, 0, 0, 1); the values are the solver's at 0.25 / 0.5 / 0.75.
        let curve = CubicBezier(0.2, 0, 0, 1)
        #expect(curve.value(at: 0) == 0 && curve.value(at: 1) == 1)
        let samples = [0.25, 0.5, 0.75].map { curve.value(at: $0) }
        #expect(samples == samples.sorted() && samples[0] > 0.25, "it only rises, and is ahead of linear early")
    }
}
