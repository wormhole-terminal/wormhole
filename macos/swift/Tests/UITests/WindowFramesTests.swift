import AppKit
import Testing
import WindowsModel

@testable import WindowUI

/// A window's place goes to the core's space (y down, primary display first) and back unchanged, whatever the displays.
@MainActor
@Suite struct WindowFramesTests {
    @Test func flippingAgainstThePrimaryDisplayIsItsOwnInverse() {
        let rect = NSRect(x: 120, y: 300, width: 900, height: 600)
        #expect(WindowFrames.flipped(WindowFrames.flipped(rect)) == rect)
        let top = WindowFrames.flipped(NSRect(x: 0, y: 0, width: 10, height: 10))
        #expect(top.maxY == NSScreen.screens.first?.frame.height ?? 0, "the bottom edge of the primary display is as far down as it goes")
    }

    @Test func theDisplaysAreTheScreensWithThePrimaryFirst() throws {
        let displays = WindowFrames.displays()
        try #require(!displays.isEmpty)
        #expect(displays.count == NSScreen.screens.count)
        #expect(displays[0].width > 0 && displays[0].height > 0)
        #expect(Set(displays.map(\.id)).count == displays.count, "each display has an id of its own")
    }

    @Test func aWindowKeepsItsPlaceThroughARecord() {
        let window = NSWindow(contentRect: NSRect(x: 200, y: 200, width: 800, height: 500), styleMask: [.titled, .resizable], backing: .buffered, defer: true)
        let frame = WindowFrames.frame(of: window, windowed: window.frame)
        #expect(frame.kind == .windowed && !frame.minimised)
        let other = NSWindow(contentRect: .zero, styleMask: [.titled, .resizable], backing: .buffered, defer: true)
        WindowFrames.place(frame, on: other)
        #expect(other.frame == window.frame)
    }
}
