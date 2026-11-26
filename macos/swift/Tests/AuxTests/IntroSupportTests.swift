import CoreGraphics
import Metal
import XCTest
@testable import AuxWindows

/// `intro-support` tells three causes of an intro that does not play apart; each answer is checked here, the ones a runner would
/// give included (no device, a source not found), by handing the decision what it would find.
final class IntroSupportTests: XCTestCase {
    private let folder = URL(fileURLWithPath: "/copy/Contents/Resources/Wormhole_AuxWindows.bundle/Contents/Resources/shader")

    private func found(_ name: String) -> URL? { folder.appendingPathComponent("\(name).metal") }

    func testEverySourceFoundAndNoDeviceSaysSoAndCompilesNothing() {
        let lines = IntroSupport.report(locate: found, device: { nil }).split(separator: "\n").map(String.init)
        let total = IntroShader.files.count
        XCTAssertEqual(lines, [
            "shaders \(total)/\(total) \(folder.path)", "metal none", "compile skipped: there is no Metal device",
        ])
    }

    func testASourceNotFoundIsNamedWhateverTheDevice() throws {
        let device = try XCTUnwrap(MTLCreateSystemDefaultDevice(), "this test needs a Metal device")
        for gpu in [device, nil] {
            let lines = IntroSupport.report(locate: { $0 == "vortex" || $0 == "warp" ? nil : self.found($0) }, device: { gpu })
                .split(separator: "\n").map(String.init)
            XCTAssertEqual(lines[0], "shaders \(IntroShader.files.count - 2)/\(IntroShader.files.count) missing vortex, warp")
            XCTAssertEqual(lines[2], "compile skipped: the sources are not found")
        }
    }

    /// The bundled sources, on this machine's device: the answer a healthy Mac gives.
    func testTheBundledSourcesCompileOnADevice() throws {
        let device = try XCTUnwrap(MTLCreateSystemDefaultDevice(), "this test needs a Metal device")
        let lines = IntroSupport.report(locate: IntroShader.url(of:), device: { device }).split(separator: "\n").map(String.init)
        XCTAssertTrue(lines[0].hasPrefix("shaders \(IntroShader.files.count)/\(IntroShader.files.count) "), lines[0])
        XCTAssertEqual(lines[1], "metal \(device.name)")
        XCTAssertEqual(lines[2], "compile ok")
    }

    func testAnErrorIsOneBoundedLine() {
        struct Failure: Error, CustomStringConvertible { var description: String { "line one\nline two\n" + String(repeating: "x", count: 900) } }
        let line = IntroSupport.describe(Failure())
        XCTAssertFalse(line.contains("\n"))
        XCTAssertEqual(line.count, 400)
        XCTAssertTrue(line.hasPrefix("line one line two xxx"))
    }

    // What a play depends on besides the shaders, as `intro-support` says it (`IntroSupport.environment`).

    private func environment(
        screens: [CGSize] = [CGSize(width: 1024, height: 768)], reduceMotion: Bool = false, background: Bool = true,
        window: Bool? = true, play: IntroPlay = .none, playing: Bool = false
    ) -> [String] {
        IntroSupport.environment(
            screens: screens, reduceMotion: reduceMotion, background: background, window: window, play: play, playing: playing
        ).split(separator: "\n").map(String.init)
    }

    func testAMachineWithNoScreenSaysSoAsTheFirstWordOfItsLine() {
        XCTAssertEqual(environment(screens: [])[0], "screens 0")
        XCTAssertEqual(environment()[0], "screens 1 1024x768")
        let two = environment(screens: [CGSize(width: 1512, height: 982), CGSize(width: 1920, height: 1080)])[0]
        XCTAssertEqual(two, "screens 2 1512x982 1920x1080")
    }

    func testReduceMotionAndWhereTheOverlayGoAreBothSaid() {
        XCTAssertEqual(Array(environment(reduceMotion: true, background: true)[1...2]), ["reduce-motion on", "overlay above the window"])
        XCTAssertEqual(Array(environment(reduceMotion: false, background: false)[1...2]), ["reduce-motion off", "overlay over the screen"])
    }

    func testTheWindowToPlayOverIsShownHiddenOrNone() {
        XCTAssertEqual(environment(window: true)[3], "window shown")
        XCTAssertEqual(environment(window: false)[3], "window hidden")
        XCTAssertEqual(environment(window: nil)[3], "window none")
    }

    func testEveryWayAPlayEndsHasItsOwnLine() {
        let lines = [IntroPlay.none, .noWindow, .failed("noDevice"), .opened(shown: true), .opened(shown: false)].map { environment(play: $0)[4] }
        XCTAssertEqual(lines, ["play none", "play no window", "play failed: noDevice", "play opened shown", "play opened hidden"])
        XCTAssertEqual(environment(playing: true)[5], "playing yes")
        XCTAssertEqual(environment(playing: false)[5], "playing no")
    }

    func testTheSixLinesFollowTheThreeOfTheShaders() {
        XCTAssertEqual(environment().count, 6)
        let report = IntroSupport.report(locate: found, device: { nil }).split(separator: "\n")
        XCTAssertEqual(report.count, 3, "the shaders' answer is unchanged: three lines, which a check reads by their first words")
    }
}
