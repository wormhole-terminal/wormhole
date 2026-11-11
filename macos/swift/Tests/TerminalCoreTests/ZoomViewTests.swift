import AppKit
import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// The view's text zoom: a window that is never shown, on the mock.
@MainActor
@Suite struct ZoomViewTests {
    private func view(inWindow: Bool = true) throws -> (MetalTerminalView, MockFrameSource, NSWindow?) {
        let configuration = RendererConfiguration(font: .system(family: "Menlo"), pointSize: 14, scale: 2)
        let frame = NSRect(x: 0, y: 0, width: 600, height: 300)
        let source = MockFrameSource(.init(scenario: .text, hertz: 0, manual: true), size: GridSize(cols: 60, rows: 12))
        let view = try MetalTerminalView(source: source, configuration: configuration, frame: frame)
        guard inWindow else { return (view, source, nil) }
        let window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = view
        view.layoutSubtreeIfNeeded()
        return (view, source, window)
    }

    @Test func aStepIsAPointAndStaysWithinEightAndThirtySix() throws {
        let (view, _, _) = try view(inWindow: false)
        view.zoom(step: 1)
        #expect(view.pointSize == 15 && view.zoom == 1)
        for _ in 0..<40 { view.zoom(step: 1) }
        #expect(view.pointSize == 36)
        for _ in 0..<80 { view.zoom(step: -1) }
        #expect(view.pointSize == 8)
        view.resetZoom()
        #expect(view.pointSize == 14 && view.zoom == 0)
    }

    @Test func withoutAWindowTheZoomCutsAndTheProgramIsToldTheNewGridOnce() throws {
        let (view, _, _) = try view(inWindow: false)
        var told: [[Int]] = []
        view.onZoomSettled = { told.append([$0, $1]) }
        view.zoom(step: 4)
        #expect(told.count == 1 && told[0][0] < 70, "fewer, bigger cells: \(told)")
        #expect(view.renderer.configuration.fonts.pointSize == 18, "cut: already drawn at the new size")
    }

    @Test func adoptingAZoomSaysNothing() throws {
        let (view, _, _) = try view()
        var told = 0
        view.onZoomSettled = { _, _ in told += 1 }
        view.setZoom(3, animated: false)
        #expect(view.renderer.configuration.fonts.pointSize == 17 && told == 0)
    }

    @Test func anEaseKeepsTheGridUntilItArrivesAndThenTellsTheProgramOnce() throws {
        let (view, _, window) = try view()
        defer { view.shutdown(); _ = window }
        guard !view.reduceMotion else { return }
        var told: [[Int]] = []
        view.onZoomSettled = { told.append([$0, $1]) }
        let grid = view.gridSize
        view.zoom(step: 6)
        #expect(view.zoomEase != nil)
        #expect(view.renderer.configuration.fonts.pointSize == 14, "nothing moved yet")
        view.advanceZoom()
        #expect(view.gridSize == grid, "the program keeps its grid while the size moves")
        // The clock is the test's, not the machine's: the suites run in parallel on the main actor.
        view.advanceZoom(at: Date().addingTimeInterval(0.05))
        let middle = view.renderer.configuration.fonts.pointSize
        #expect(middle > 14 && middle < 20 && middle == TextZoomEase.quantise(middle), "a quarter-point size on the way: \(middle)")
        #expect(view.gridSize == grid && told.isEmpty)
        view.advanceZoom(at: Date().addingTimeInterval(0.5))
        #expect(view.zoomEase == nil)
        #expect(view.renderer.configuration.fonts.pointSize == 20)
        #expect(told.count == 1 && view.gridSize != grid, "told once, on arrival")
    }

    @Test func aSecondPressGoesOnFromWhereTheFirstGot() throws {
        let (view, _, window) = try view()
        defer { view.shutdown(); _ = window }
        guard !view.reduceMotion else { return }
        view.zoom(step: 1)
        view.zoom(step: 1)
        #expect(view.zoom == 2 && view.zoomEase != nil)
        #expect(view.pointSize == 16)
    }

    @Test func newSettingsAreACutAndKeepTheZoom() throws {
        let (view, _, _) = try view(inWindow: false)
        view.zoom(step: 2)
        view.setFonts(TerminalFontSettings(font: .system(family: "Menlo"), pointSize: 12, lineHeight: 1.2, ligatures: false))
        #expect(view.renderer.configuration.fonts.pointSize == 14, "12 from Settings plus the zoom of 2")
        #expect(view.fontSettings.pointSize == 12 && view.zoom == 2)
    }
}
