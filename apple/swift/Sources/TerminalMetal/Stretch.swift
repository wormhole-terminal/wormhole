// How much bigger than a normal cell a row draws its glyphs: DECDWL rows are twice as wide, DECDHL rows (each
// half of a line) twice as wide and as tall. A glyph is rasterised at that stretch, not scaled from the
// normal one, so its edges stay sharp (GPUI paints the outline stretched, `render/sized.rs`).

import CoreGraphics
import TerminalCore

enum Stretch: UInt64 {
    case normal = 0
    case wide
    case tall

    init(_ size: LineSize) {
        switch size {
        case .single: self = .normal
        case .doubleWidth: self = .wide
        case .doubleHeightTop, .doubleHeightBottom: self = .tall
        }
    }

    var x: Int { self == .normal ? 1 : 2 }
    var y: Int { self == .tall ? 2 : 1 }

    /// The atlas key of a glyph of a font (`FontSet.id`) in this stretch.
    func key(font: UInt16, glyph: UInt16) -> UInt64 { rawValue << 32 | UInt64(font) << 16 | UInt64(glyph) }

    /// The atlas key of a box-drawing, block or braille cell: past every font id.
    func boxKey(_ scalar: UInt32) -> UInt64 { rawValue << 32 | 0xFFFF_0000 | UInt64(scalar & 0xFFFF) }
}

extension CellMetrics {
    /// A cell as a stretched row draws it.
    func stretched(_ stretch: Stretch) -> CellMetrics {
        guard stretch != .normal else { return self }
        let (sx, sy) = (stretch.x, stretch.y)
        return CellMetrics(
            width: width * sx, height: height * sy, baseline: baseline * sy, underlineTop: underlineTop * sy,
            lineThickness: lineThickness * sy, strikeTop: strikeTop * sy)
    }
}
