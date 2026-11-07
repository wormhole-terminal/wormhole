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

    /// `stretch` scales the outline (a double-width or double-height row); the box is scaled with it.
    static func rasterize(font: CTFont, glyph: CGGlyph, stretch: Stretch = .normal) -> Raster {
        var glyphs = [glyph]
        var box = CGRect.zero
        CTFontGetBoundingRectsForGlyphs(font, .default, &glyphs, &box, 1)
        let (sx, sy) = (CGFloat(stretch.x), CGFloat(stretch.y))
        box = CGRect(x: box.minX * sx, y: box.minY * sy, width: box.width * sx, height: box.height * sy)
        guard !box.isEmpty, box.width.isFinite, box.height.isFinite else { return .empty }
        // One pixel of margin each side: antialiasing spills past the outline's own bounds.
        let x0 = Int(box.minX.rounded(.down)) - 1, y0 = Int(box.minY.rounded(.down)) - 1
        let x1 = Int(box.maxX.rounded(.up)) + 1, y1 = Int(box.maxY.rounded(.up)) + 1
        let width = x1 - x0, height = y1 - y0
        guard width > 0, height > 0, width <= 512, height <= 512 else { return .empty }
        // A stretched glyph is drawn at the origin of a scaled, shifted space, so the stretch applies to the
        // outline; a normal one at its offset (the same pixels as before stretch existed).
        var position = stretch == .normal ? CGPoint(x: -x0, y: -y0) : .zero
        func place(_ context: CGContext) {
            guard stretch != .normal else { return }
            context.translateBy(x: CGFloat(-x0), y: CGFloat(-y0))
            context.scaleBy(x: sx, y: sy)
        }

        if FontSet.hasColorGlyphs(font) {
            var pixels = [UInt8](repeating: 0, count: width * height * 4)
            let info = CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
            pixels.withUnsafeMutableBytes { raw in
                guard let context = CGContext(
                    data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                    space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: info)
                else { return }
                place(context)
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
                place(context)
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
