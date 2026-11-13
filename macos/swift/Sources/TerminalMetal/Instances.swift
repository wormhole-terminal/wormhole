// What the GPU draws, as bytes: instance structs the vertex shaders read, and the geometry a row
// keeps between frames so an unchanged row costs a copy and not a rebuild.

import Foundation

/// A solid rectangle in pixels, top-left origin: a background run, an underline, a cursor bar.
/// 24 bytes; the same fields in the same order as `SolidInstance` in `Shaders.swift`.
struct SolidInstance {
    var x: Float
    var y: Float
    var width: Float
    var height: Float
    /// The cell's colour, unresolved (tag << 24 | value); the shader resolves it against the palette.
    var color: UInt32
    var style: UInt32

    static let patternShift: UInt32 = 0
    /// `style` bits 0...3: 0 solid, 1 dotted, 2 dashed, 3 curly. Bit 8: dim.
    static let dotted: UInt32 = 1
    static let dashed: UInt32 = 2
    static let curly: UInt32 = 3
    static let dimBit: UInt32 = 1 << 8
    /// Bit 9: a background that stays inside the grid (the block cursor), where the others at an edge run on into the padding.
    static let fixedBit: UInt32 = 1 << 9
}

/// A glyph from the atlas: where to put it and what to sample. 32 bytes.
struct GlyphInstance {
    var x: Float
    var y: Float
    var width: Float
    var height: Float
    var atlasX: UInt16
    var atlasY: UInt16
    var atlasWidth: UInt16
    var atlasHeight: UInt16
    var color: UInt32
    /// Bit 0: dim. Bit 1: a colour glyph, drawn as sampled. Bit 2: blinking text.
    var flags: UInt32

    static let dimBit: UInt32 = 1
    static let colorBit: UInt32 = 2
    static let blinkBit: UInt32 = 4
}

/// One row's instances. A class so the builder appends in place and the store hands out references
/// without copying arrays.
final class RowGeometry {
    var backgrounds = ContiguousArray<SolidInstance>()
    var glyphs = ContiguousArray<GlyphInstance>()
    var decorations = ContiguousArray<SolidInstance>()
    /// Some glyph of the row blinks: the view runs the blink clock only while any row says so.
    var blinks = false

    func removeAll() {
        blinks = false
        backgrounds.removeAll(keepingCapacity: true)
        glyphs.removeAll(keepingCapacity: true)
        decorations.removeAll(keepingCapacity: true)
    }
}
