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
}
