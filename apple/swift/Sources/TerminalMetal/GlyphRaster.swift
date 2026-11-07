// Rasterising one glyph with CoreText into a bitmap the atlas stores: coverage in 8 bits for ordinary
// glyphs, premultiplied BGRA for colour ones. Called only on an atlas miss.

import CoreGraphics
import CoreText
import Foundation

struct Raster {
    var width: Int
    var height: Int
    /// From the cell's left edge to the bitmap's left edge (pixels; negative for an overhang).
    var left: Int
    /// From the baseline up to the bitmap's top edge.
    var top: Int
    var isColor: Bool
    /// Top row first; `width * height` bytes, or four times that for colour.
    var pixels: [UInt8]

    static let empty = Raster(width: 0, height: 0, left: 0, top: 0, isColor: false, pixels: [])
}

enum GlyphRaster {
    /// Whether to let CoreGraphics thicken stems the way the system draws text.
    nonisolated(unsafe) static var fontSmoothing = true

    static func rasterize(font: CTFont, glyph: CGGlyph) -> Raster {
        var glyphs = [glyph]
        var box = CGRect.zero
        CTFontGetBoundingRectsForGlyphs(font, .default, &glyphs, &box, 1)
        guard !box.isEmpty, box.width.isFinite, box.height.isFinite else { return .empty }
        // One pixel of margin each side: antialiasing spills past the outline's own bounds.
        let x0 = Int(box.minX.rounded(.down)) - 1, y0 = Int(box.minY.rounded(.down)) - 1
        let x1 = Int(box.maxX.rounded(.up)) + 1, y1 = Int(box.maxY.rounded(.up)) + 1
        let width = x1 - x0, height = y1 - y0
        guard width > 0, height > 0, width <= 512, height <= 512 else { return .empty }
        var position = CGPoint(x: -x0, y: -y0)

        if FontSet.hasColorGlyphs(font) {
            var pixels = [UInt8](repeating: 0, count: width * height * 4)
            let info = CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
            pixels.withUnsafeMutableBytes { raw in
                guard let context = CGContext(
                    data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                    space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: info)
                else { return }
                CTFontDrawGlyphs(font, &glyphs, &position, 1, context)
            }
            return Raster(width: width, height: height, left: x0, top: y1, isColor: true, pixels: pixels)
        }
        return Raster(
            width: width, height: height, left: x0, top: y1, isColor: false,
            pixels: coverage(width: width, height: height) { context in
                context.setFillColor(gray: 1, alpha: 1)
                context.setShouldSmoothFonts(fontSmoothing)
                context.setShouldAntialias(true)
                CTFontDrawGlyphs(font, &glyphs, &position, 1, context)
            })
    }

    /// Draw white on black into an 8-bit grey bitmap and return it top row first: the grey value is the
    /// coverage. (An alpha-only context would do the same, but only a grey one gets the system's stem
    /// darkening.)
    static func coverage(width: Int, height: Int, _ draw: (CGContext) -> Void) -> [UInt8] {
        var pixels = [UInt8](repeating: 0, count: width * height)
        pixels.withUnsafeMutableBytes { raw in
            guard let context = CGContext(
                data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width,
                space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)
            else { return }
            draw(context)
        }
        return pixels
    }
}
