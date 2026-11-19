// Frames of an intro drawn off screen over a stand-in screen and window, for the control folder's `intro-frames <style> [count]`
// (GPUI's `intro::pictures::frames`): the shader's own pictures, no window needed. Written as BMP files, `intro-<style>-<n>.bmp`.

import AppKit
import AuxModel
import Foundation

public enum IntroFrames {
    /// Draws `count` frames of `style` (spread across its run) into `directory`; the files written, in order.
    @MainActor
    public static func write(_ style: IntroStyle, count: Int, colors: IntroColors, to directory: URL) throws -> [URL] {
        let renderer = try IntroRenderer()
        let (width, height) = (960, 600)
        let screen = SIMD2<Float>(Float(width), Float(height))
        let size: SIMD2<Float> = [420, 340]
        let origin = (screen - size) / 2
        let landing = Landing(centre: [screen.x / 2, origin.y + 70], size: 96)
        let base = IntroUniforms(screen: screen, origin: origin, size: size, scale: 1, colors: colors)
        var written: [URL] = []
        for index in 0..<count {
            let t = (Float(index) + 0.5) / Float(count)
            let time = t * style.seconds
            let (uniforms, trail) = base.at(style, t: t, time: time, start: .hidden).withShip(style, ship: true, time: time, screen: screen, landing: landing)
            guard var rgba = renderer.pixels(width: width, height: height, uniforms: uniforms, trail: trail) else { continue }
            // Over a dark desk colour, so transparency reads in the picture.
            for at in stride(from: 0, to: rgba.count, by: 4) {
                let alpha = Int(rgba[at + 3])
                for channel in 0..<3 { rgba[at + channel] = UInt8(min(255, Int(rgba[at + channel]) + (24 * (255 - alpha)) / 255)) }
                rgba[at + 3] = 255
            }
            guard let rep = NSBitmapImageRep(
                bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: width * 4, bitsPerPixel: 32),
                let data = rep.bitmapData
            else { continue }
            rgba.withUnsafeBufferPointer { data.update(from: $0.baseAddress!, count: $0.count) }
            guard let bmp = rep.representation(using: .bmp, properties: [:]) else { continue }
            let url = directory.appendingPathComponent("intro-\(style.rawValue)-\(index).bmp")
            try bmp.write(to: url)
            written.append(url)
        }
        return written
    }
}
