import AppKit
import Testing
import TerminalCore

@testable import TerminalMetal

/// A terminal view in a window that is never shown, on the mock, driven by synthetic `NSEvent`s: the view's own
/// code runs (event handlers, input client, menus), and what it drew is read back from a texture.
@MainActor
final class ViewHarness {
    let source: MockFrameSource
    let view: MetalTerminalView
    let window: NSWindow
    let interaction: MockInteraction
    let cell: CGSize

    init(cols: Int = 40, rows: Int = 10, text: String = "") throws {
        let renderer = RendererConfiguration(font: .system(family: "Menlo"), pointSize: 13, scale: 2)
        let probe = try MetalRenderer(configuration: renderer)
        cell = CGSize(width: CGFloat(probe.cellPixels.width) / 2, height: CGFloat(probe.cellPixels.height) / 2)
        let frame = NSRect(x: 0, y: 0, width: cell.width * CGFloat(cols), height: cell.height * CGFloat(rows))
        source = MockFrameSource(.init(scenario: .cursor, hertz: 0, manual: true), size: GridSize(cols: cols, rows: rows))
        view = try MetalTerminalView(source: source, configuration: renderer, frame: frame)
        view.fixedGrid = (cols, rows)
        window = NSWindow(contentRect: frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = view
        view.layoutSubtreeIfNeeded()
        source.write(Data(text.utf8))
        view.drawFrame(force: true)
        interaction = source.interaction as! MockInteraction
    }

    deinit { MainActor.assumeIsolated { view.shutdown() } }

    /// The centre of a cell, in the view's coordinates (the view is flipped, the grid at its top-left).
    func point(row: Int, column: Int, half: CGFloat = 0.5) -> CGPoint {
        CGPoint(x: (CGFloat(column) + half) * cell.width, y: (CGFloat(row) + 0.5) * cell.height)
    }

    func mouse(_ type: NSEvent.EventType, at point: CGPoint, clicks: Int = 1, flags: NSEvent.ModifierFlags = []) -> NSEvent {
        let inWindow = view.convert(point, to: nil)
        return NSEvent.mouseEvent(
            with: type, location: inWindow, modifierFlags: flags, timestamp: 0, windowNumber: window.windowNumber, context: nil,
            eventNumber: 0, clickCount: clicks, pressure: 1)!
    }

    func drag(from: CGPoint, to: CGPoint, clicks: Int = 1, flags: NSEvent.ModifierFlags = []) {
        view.mouseDown(with: mouse(.leftMouseDown, at: from, clicks: clicks, flags: flags))
        view.mouseDragged(with: mouse(.leftMouseDragged, at: to, flags: flags))
        view.mouseUp(with: mouse(.leftMouseUp, at: to, flags: flags))
        view.drawFrame(force: true)
    }

    /// The pixel at the middle of a cell, as drawn.
    func pixel(row: Int, column: Int) -> (r: UInt8, g: UInt8, b: UInt8) {
        let image = view.renderer.renderToImage()
        let size = view.renderer.cellPixels
        // A corner of the cell: the glyph, if any, is in the middle.
        return image.pixel(column * size.width + 1, row * size.height + 1)
    }
}
