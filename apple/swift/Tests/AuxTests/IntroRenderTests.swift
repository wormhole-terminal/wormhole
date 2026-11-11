import AppKit
import DesignSystem
import XCTest
@testable import AuxModel
@testable import AuxWindows

/// The shader compiles at run time, so a mistake in it would only show as an intro that never plays: compile it,
/// draw a frame of each style, and write pictures to look at.
final class IntroRenderTests: XCTestCase {
    static let outDirectory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["AUX_SNAPSHOTS"]
            ?? NSTemporaryDirectory() + "wormhole-swift-shots/swift-g-aux")

    func testUniformsMatchTheShadersLayout() {
        XCTAssertEqual(MemoryLayout<IntroUniforms>.stride, 160)
        XCTAssertEqual(MemoryLayout<IntroUniforms>.offset(of: \.run), 64)
        XCTAssertEqual(MemoryLayout<IntroUniforms>.offset(of: \.c3), 144)
    }

    func testEveryStyleDrawsSomething() throws {
        let renderer = try IntroRenderer()
        let tokens = try XCTUnwrap(ThemeLibrary.bundled.theme(id: ThemeLibrary.bundled.defaultDark)).tokens()
        let screen: SIMD2<Float> = [256, 240]
        let base = IntroUniforms(screen: screen, origin: [96, 96], size: [64, 48], scale: 1, colors: IntroColors(tokens: tokens))
        for style in IntroStyle.allCases {
            let (u, trail) = base.at(style, t: 0.5, time: 0.5 * style.seconds, start: .shown)
                .withShip(style, ship: true, time: 0.5 * style.seconds, screen: screen, landing: Landing(centre: [128, 80], size: 24))
            let pixels = try XCTUnwrap(renderer.pixels(width: 256, height: 240, uniforms: u, trail: trail))
            XCTAssertTrue(stride(from: 3, to: pixels.count, by: 4).contains { pixels[$0] > 0 }, "\(style) drew nothing")
        }
    }

    /// Voyage at the moments that matter, over a screen-sized stand-in: PNGs to look at.
    func testWritePictures() throws {
        let renderer = try IntroRenderer()
        let tokens = try XCTUnwrap(ThemeLibrary.bundled.theme(id: ThemeLibrary.bundled.defaultDark)).tokens()
        let (w, h) = (960, 600)
        let screen = SIMD2<Float>(Float(w), Float(h))
        let size: SIMD2<Float> = [420, 340]
        let origin = (screen - size) / 2
        let landing = Landing(centre: [screen.x / 2, origin.y + 70], size: 96)
        let base = IntroUniforms(screen: screen, origin: origin, size: size, scale: 1, colors: IntroColors(tokens: tokens))
        try FileManager.default.createDirectory(at: Self.outDirectory, withIntermediateDirectories: true)
        let moments: [(IntroStyle, Float)] = [
            (.voyage, 1.0), (.voyage, 3.0), (.voyage, 4.9), (.voyage, 5.6), (.voyage, 6.2), (.voyage, 7.0),
            (.aurora, 3.0), (.emerge, 1.9), (.vortex, 1.5), (.ftl, 4.0), (.collapse, 2.5), (.glow, 0.5),
        ]
        for (style, time) in moments {
            let t = time / style.seconds
            let (u, trail) = base.at(style, t: t, time: time, start: .hidden)
                .withShip(style, ship: true, time: time, screen: screen, landing: landing)
            let pixels = try XCTUnwrap(renderer.pixels(width: w, height: h, uniforms: u, trail: trail))
            try write(pixels, w, h, to: "intro-\(style.rawValue)-\(Int(time * 10))")
        }
    }

    private func write(_ rgba: [UInt8], _ w: Int, _ h: Int, to name: String) throws {
        // Composite over the desk colour a window would have under it, so transparency reads in the PNG.
        var data = rgba
        for i in stride(from: 0, to: data.count, by: 4) {
            let a = Int(data[i + 3])
            for c in 0..<3 { data[i + c] = UInt8(min(255, Int(data[i + c]) + (24 * (255 - a)) / 255)) }
            data[i + 3] = 255
        }
        let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: w, pixelsHigh: h, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
            isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: w * 4, bitsPerPixel: 32)!
        data.withUnsafeBufferPointer { rep.bitmapData!.update(from: $0.baseAddress!, count: $0.count) }
        try rep.representation(using: .png, properties: [:])!.write(to: Self.outDirectory.appendingPathComponent(name + ".png"))
    }
}
