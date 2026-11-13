import AppKit
import DesignSystem
import SwiftUI
import XCTest
@testable import AuxModel
@testable import AuxWindows

/// The overlay and the player over a window that is never shown: the window's fade and scale follow the style's
/// motion, a skip settles it, closing puts everything back, and Reduce Motion plays the still glow.
@MainActor
final class IntroOverlayTests: XCTestCase {
    private func makeWindow() throws -> NSWindow {
        _ = NSApplication.shared
        try XCTSkipIf(NSScreen.main == nil, "no screen: the overlay covers one")
        let window = NSWindow(contentRect: CGRect(x: 200, y: 200, width: 480, height: 360), styleMask: [.titled, .fullSizeContentView], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Color.clear)
        window.contentView?.wantsLayer = true
        return window
    }

    private func colors() throws -> IntroColors {
        IntroColors(tokens: try XCTUnwrap(ThemeLibrary.bundled.theme(id: "wormhole.dark")).tokens())
    }

    func testTheWindowLeavesAndArrivesAndClosingPutsItBack() throws {
        let window = try makeWindow()
        window.alphaValue = 1
        let setup = IntroSetup(style: .voyage, colors: try colors(), start: .hidden, lights: false, ship: true, immersive: false)
        let overlay = try IntroOverlay(over: window, setup: setup, landing: { nil })
        XCTAssertEqual(window.alphaValue, 0, "a first launch's window is out of sight until the wormhole delivers it")
        overlay.frame(IntroFrame(t: 0.8, time: 0.8 * Voyage.seconds, lights: nil, settling: nil))
        XCTAssertEqual(window.alphaValue, 1, accuracy: 0.01, "arrived")
        overlay.frame(IntroFrame(t: 0.58, time: 0.58 * Voyage.seconds, lights: nil, settling: nil))
        let scaled = try XCTUnwrap(window.contentView?.superview?.subviews.compactMap(\.layer).first)
        XCTAssertNotEqual(scaled.transform.m11, 1, "the window spring settles about full size")
        overlay.close()
        XCTAssertEqual(window.alphaValue, 1)
        XCTAssertTrue(CATransform3DIsIdentity(scaled.transform))
    }

    func testASkipSettlesTheWindowBackToRest() throws {
        let window = try makeWindow()
        let setup = IntroSetup(style: .emerge, colors: try colors(), start: .shown, lights: false, ship: true, immersive: false)
        let overlay = try IntroOverlay(over: window, setup: setup, landing: { nil })
        let from = overlay.motion(at: 0.2)
        overlay.frame(IntroFrame(t: 0.2, time: 0.8, lights: nil, settling: (from, 0.5)))
        XCTAssertGreaterThan(window.alphaValue, CGFloat(from.alpha))
        overlay.frame(IntroFrame(t: 0.2, time: 0.8, lights: nil, settling: (from, 1)))
        XCTAssertEqual(window.alphaValue, 1, accuracy: 0.001)
        overlay.close()
    }

    func testReduceMotionPlaysTheStillGlowAndThePageIsThere() throws {
        let window = try makeWindow()
        let player = IntroPlayer(reduceMotion: { true })
        var options = IntroOptions()
        options.immersive = false
        player.play(.voyage, over: window, options: options, colors: try colors())
        XCTAssertEqual(player.progress?.style, .glow)
        XCTAssertEqual(player.entrance, .done)
        XCTAssertFalse(player.isEntering)
        player.skip()
        player.stop()
        XCTAssertNil(player.progress)
    }

    func testAMovingStyleHoldsThePageBackUntilTheWindowArrives() throws {
        let window = try makeWindow()
        let player = IntroPlayer(reduceMotion: { false })
        var options = IntroOptions()
        (options.immersive, options.start) = (false, .hidden)
        player.play(.voyage, over: window, options: options, colors: try colors())
        XCTAssertEqual(player.progress?.style, .voyage)
        XCTAssertTrue(player.isEntering)
        XCTAssertEqual(player.entrance, IntroStyle.voyage.entrance(at: 0))
        player.stop()
        XCTAssertEqual(window.alphaValue, 1)
    }

    /// Whether a display link of the main screen ticks within a third of a second.
    private func screenLinkTicks() async -> Bool {
        guard let screen = NSScreen.main else { return false }
        let probe = TickProbe()
        let link = screen.displayLink(target: probe, selector: #selector(TickProbe.tick(_:)))
        link.add(to: .main, forMode: .common)
        defer { link.invalidate() }
        try? await Task.sleep(for: .milliseconds(300))
        return probe.ticks > 0
    }

    /// The still glow on the real display link, over a window that sits far off any screen (so nothing is seen):
    /// it ticks, finishes on its own and calls back once.
    func testTheGlowRunsOnTheDisplayLinkAndFinishes() async throws {
        let window = try makeWindow()
        window.setFrameOrigin(NSPoint(x: -30000, y: -30000))
        window.orderBack(nil)
        defer { window.orderOut(nil) }
        // A runner's display does not always drive a display link (the test failed there one run in a few with a
        // progress of 0.0): where the screen's own link does not tick, the player's cannot, and that says nothing of it.
        let ticks = await screenLinkTicks()
        try XCTSkipUnless(ticks, "no display link ticks on this screen")
        let player = IntroPlayer(reduceMotion: { true })
        var finished = 0
        player.onFinish = { finished += 1 }
        var options = IntroOptions()
        options.immersive = false
        player.play(.voyage, over: window, options: options, colors: try colors())
        try await Task.sleep(for: .milliseconds(500))
        XCTAssertGreaterThan(player.progress?.seconds ?? 0, 0.1, "the display link ticks")
        try await Task.sleep(for: .milliseconds(1200))
        XCTAssertNil(player.progress, "the run is over")
        XCTAssertEqual(finished, 1)
    }
}

/// Counts a display link's ticks.
private final class TickProbe: NSObject {
    var ticks = 0
    @objc func tick(_ link: CADisplayLink) { ticks += 1 }
}
