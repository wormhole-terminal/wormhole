// Rasterising one glyph with CoreText into a bitmap the atlas stores: coverage in 8 bits for ordinary glyphs,
// premultiplied BGRA for colour ones. Called only on an atlas miss.
//
// Coverage glyphs are made the way GPUI's text system makes them (`gpui-pre-apple` `rasterize_glyph`), so the two apps
// put the same ink on screen: an alpha-only bitmap, the font at its point size under the screen's scale, the glyph
// shifted by a quarter-pixel `variant` (a cell 16.8 px wide puts glyphs between pixels), subpixel positioning on and
// quantisation off. CoreGraphics' font smoothing thickens stems by an amount that follows the fill colour's
// luminance; GPUI turns it on only for text whose colour is light enough (`dilation` 1...4, from `level(of:)`) and
// fills with that luminance, so light text on a dark terminal is drawn bolder and dark text on a light one is not.

import CoreGraphics
import CoreText
import Foundation

struct Raster {
    var width: Int
    var height: Int
    /// From the glyph's origin to the bitmap's left edge (pixels; negative for an overhang).
    var left: Int
    /// From the baseline up to the bitmap's top edge.
    var top: Int
    var isColor: Bool
    /// Top row first; `width * height` bytes, or four times that for colour.
    var pixels: [UInt8]

    static let empty = Raster(width: 0, height: 0, left: 0, top: 0, isColor: false, pixels: [])
}

enum GlyphRaster {
    /// Quarter pixels, as GPUI's `SUBPIXEL_VARIANTS_X`; rows stay on whole pixels.
    static let variants = 4

    /// The stem dilation GPUI picks for text of this colour (`glyph_dilation_for_color`): its luminance in quarters,
    /// or 0 when the person turned font smoothing off (`AppleFontSmoothing` 0).
    static func level(of rgb: SIMD3<UInt8>) -> Int {
        guard smoothingAllowed else { return 0 }
        let luminance = (0.2126 * Float(rgb.x) + 0.7152 * Float(rgb.y) + 0.0722 * Float(rgb.z)) / 255
        return min(4, max(0, Int((4 * luminance + 0.5).rounded(.down))))
    }

    private static let smoothingAllowed: Bool = {
        let value = CFPreferencesCopyAppValue("AppleFontSmoothing" as CFString, kCFPreferencesCurrentApplication)
        return (value as? NSNumber)?.intValue != 0
    }()

    /// `font` is the face at the drawn size (points x `scale`); `stretch` scales the outline (a double-width or
    /// double-height row). A colour glyph ignores `variant` and `dilation`.
    static func rasterize(font: CTFont, glyph: CGGlyph, scale: CGFloat, stretch: Stretch = .normal, variant: Int = 0, dilation: Int = 0) -> Raster {
        // GPUI draws the face at its point size under the screen's scale (the stretch goes into the same transform) and
        // measures it so too: Apple Color Emoji picks its bitmap strike by the face's size, so a 14 pt emoji at 2x is larger
        // than a 28 pt one.
        let face = CTFontCreateCopyWithAttributes(font, CTFontGetSize(font) / scale, nil, nil)
        var glyphs = [glyph]
        var box = CGRect.zero
        CTFontGetBoundingRectsForGlyphs(face, .default, &glyphs, &box, 1)
        let (sx, sy) = (CGFloat(stretch.x) * scale, CGFloat(stretch.y) * scale)
        box = CGRect(x: box.minX * sx, y: box.minY * sy, width: box.width * sx, height: box.height * sy)
        guard !box.isEmpty, box.width.isFinite, box.height.isFinite else { return .empty }
        let color = FontSet.hasColorGlyphs(font)
        let shift = color ? 0 : variant
        // One pixel of margin each side: antialiasing spills past the outline's own bounds; one more across for a shift.
        let x0 = Int(box.minX.rounded(.down)) - 1, y0 = Int(box.minY.rounded(.down)) - 1
        let x1 = Int(box.maxX.rounded(.up)) + 1 + (shift > 0 ? 1 : 0), y1 = Int(box.maxY.rounded(.up)) + 1
        let width = x1 - x0, height = y1 - y0
        guard width > 0, height > 0, width <= 512, height <= 512 else { return .empty }
        var position = CGPoint(x: CGFloat(shift) / CGFloat(variants) / scale, y: 0)
        func place(_ context: CGContext) {
            context.translateBy(x: CGFloat(-x0), y: CGFloat(-y0))
            context.scaleBy(x: sx, y: sy)
        }

        if color {
            var pixels = [UInt8](repeating: 0, count: width * height * 4)
            let info = CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
            pixels.withUnsafeMutableBytes { raw in
                guard let context = CGContext(
                    data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                    space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: info)
                else { return }
                place(context)
                CTFontDrawGlyphs(face, &glyphs, &position, 1, context)
            }
            return Raster(width: width, height: height, left: x0, top: y1, isColor: true, pixels: pixels)
        }
        var pixels = [UInt8](repeating: 0, count: width * height)
        pixels.withUnsafeMutableBytes { raw in
            guard let context = CGContext(
                data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width,
                space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.alphaOnly.rawValue)
            else { return }
            place(context)
            context.setTextDrawingMode(.fill)
            context.setAllowsAntialiasing(true)
            context.setShouldAntialias(true)
            context.setAllowsFontSubpixelPositioning(true)
            context.setShouldSubpixelPositionFonts(true)
            context.setAllowsFontSubpixelQuantization(false)
            context.setShouldSubpixelQuantizeFonts(false)
            if dilation > 0 {
                context.setShouldSmoothFonts(true)
                context.setFillColor(gray: CGFloat(dilation) * 0.25, alpha: 1)
            } else {
                context.setFillColor(gray: 0, alpha: 1)
            }
            CTFontDrawGlyphs(face, &glyphs, &position, 1, context)
        }
        return Raster(width: width, height: height, left: x0, top: y1, isColor: false, pixels: pixels)
    }

    /// Draw white on black into an 8-bit grey bitmap and return it top row first: the grey value is the coverage (box
    /// drawing's exact shapes).
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
