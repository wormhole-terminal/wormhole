import AppKit
import AppLogging
import AuxModel
import DesignSystem
import Synchronization
import SwiftUI
import XCTest
@testable import AuxWindows

/// `IntroPlayer.play` leaves nothing to look at when it does not show, so each way it can end with no overlay is checked to say so:
/// in `lastPlay` (`intro-support` reads it) and in the log. The overlay that cannot be made is handed in; one that opens over a
/// window that is on screen, and over one that is not, is the real thing.
@MainActor
final class IntroPlayTests: XCTestCase {
    /// The lines the log was given, from whichever thread.
    private final class Lines: Sendable {
        let held = Mutex<[String]>([])
    }

    private let logged = Lines()

    override func setUp() async throws {
        // The sink is the whole process's: it holds this test's lines until it is taken away again.
        AppLog.install { [logged] level, message in logged.held.withLock { $0.append("\(level): \(message)") } }
    }

    override func tearDown() async throws {
        AppLog.install(nil)
    }

    private func lines() -> [String] { logged.held.withLock { $0 } }

    private func colors() throws -> IntroColors {
        IntroColors(tokens: try XCTUnwrap(ThemeLibrary.bundled.theme(id: "wormhole.dark")).tokens())
    }

    private func makeWindow() throws -> NSWindow {
        _ = NSApplication.shared
        try XCTSkipIf(NSScreen.main == nil, "no screen: the overlay covers one")
        let window = NSWindow(contentRect: CGRect(x: 200, y: 200, width: 480, height: 360), styleMask: [.titled, .fullSizeContentView], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Color.clear)
        window.contentView?.wantsLayer = true
        return window
    }

    private var inTheBackground: IntroOptions {
        var options = IntroOptions()
        options.immersive = false
        return options
    }

    func testNothingAskedYetSaysNone() {
        XCTAssertEqual(IntroPlayer(reduceMotion: { false }).lastPlay, .none)
    }

    func testNoWindowIsSaidAndLogged() throws {
        let player = IntroPlayer(reduceMotion: { false })
        player.play(.ftl, over: nil, colors: try colors())
        XCTAssertEqual(player.lastPlay, .noWindow)
        XCTAssertFalse(player.isPlaying)
        let lines = lines()
        XCTAssertEqual(lines, ["warn: the intro did not play: there is no window to play it over"])
    }

    func testAnOverlayThatCannotBeMadeIsSaidWithItsErrorAndLogged() throws {
        let window = NSWindow(contentRect: .zero, styleMask: [.titled], backing: .buffered, defer: true)
        let player = IntroPlayer(reduceMotion: { false }) { _, _, _ in throw IntroRenderer.SetupError.noDevice }
        player.play(.ftl, over: window, colors: try colors())
        XCTAssertEqual(player.lastPlay, .failed("noDevice"))
        XCTAssertNil(player.progress)
        let lines = lines()
        XCTAssertEqual(lines, ["warn: the intro did not play: noDevice"])
    }

    func testAnOverlayOverAWindowOnScreenIsShown() throws {
        let window = try makeWindow()
        window.setFrameOrigin(NSPoint(x: -30000, y: -30000))
        window.orderBack(nil)
        defer { window.orderOut(nil) }
        let player = IntroPlayer(reduceMotion: { true })
        player.play(.ftl, over: window, options: inTheBackground, colors: try colors())
        defer { player.stop() }
        XCTAssertEqual(player.lastPlay, .opened(shown: true))
        XCTAssertTrue(player.isPlaying)
        let lines = lines()
        XCTAssertEqual(lines, [], "an intro that shows says nothing")
    }

    func testAnOverlayOverAWindowThatIsNotOnScreenIsHiddenAndLogged() throws {
        let window = try makeWindow()
        XCTAssertFalse(window.isVisible)
        let player = IntroPlayer(reduceMotion: { true })
        player.play(.ftl, over: window, options: inTheBackground, colors: try colors())
        defer { player.stop() }
        XCTAssertEqual(player.lastPlay, .opened(shown: false))
        let lines = lines()
        XCTAssertEqual(lines, ["warn: the intro's overlay opened but is not on screen (the window it plays over is not on screen)"])
    }

    func testReduceMotionIsAsThePlayerReadsIt() {
        XCTAssertTrue(IntroPlayer(reduceMotion: { true }).reducesMotion)
        XCTAssertFalse(IntroPlayer(reduceMotion: { false }).reducesMotion)
    }
}
