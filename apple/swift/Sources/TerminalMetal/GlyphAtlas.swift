// The glyph atlas: shelf-packed textures the shader samples. Coverage glyphs go to an R8 texture and are
// tinted by the cell's colour; colour glyphs (emoji) to a BGRA one and drawn as they are. A glyph is rasterised
// once per quarter-pixel variant and stem dilation it is drawn at (`WordCache.entry`), as GPUI's atlas keeps them.
//
// When a texture is full the whole atlas is cleared (`generation` changes and the owner drops everything
// that held an entry); a terminal's working set is a few hundred glyphs, so this is rare, and it counts.

import Metal

/// Where a glyph is in its atlas texture and how it sits against its cell.
struct AtlasEntry {
    var x: UInt16
    var y: UInt16
    var width: UInt16
    var height: UInt16
    /// Pixels from the cell's left edge to the bitmap's left edge.
    var left: Int16
    /// Pixels from the baseline up to the bitmap's top edge.
    var top: Int16
    var isColor: Bool

    var isEmpty: Bool { width == 0 || height == 0 }
}

final class GlyphAtlas {
    static let maskSize = 2048
    static let colorSize = 1024

    let maskTexture: MTLTexture
    let colorTexture: MTLTexture
    private(set) var entries: ContiguousArray<AtlasEntry> = []
    private var index: [UInt64: Int32] = [:]
    private var mask = ShelfPacker(size: GlyphAtlas.maskSize)
    private var color = ShelfPacker(size: GlyphAtlas.colorSize)
    /// How many times the atlas filled up and started again.
    private(set) var resets = 0
    private(set) var generation = 0
    /// Glyphs rasterised so far (a counting guard for the tests and the measuring mode).
    private(set) var rasterised = 0

    init(device: MTLDevice) {
        func texture(_ format: MTLPixelFormat, _ size: Int) -> MTLTexture {
            let descriptor = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: format, width: size, height: size, mipmapped: false)
            descriptor.storageMode = .shared
            descriptor.usage = .shaderRead
            return device.makeTexture(descriptor: descriptor)!
        }
        maskTexture = texture(.r8Unorm, Self.maskSize)
        colorTexture = texture(.bgra8Unorm, Self.colorSize)
    }

    /// The entry for `key`, or `nil` if it was never added.
    @inline(__always)
    func lookup(_ key: UInt64) -> Int32? { index[key] }

    @inline(__always)
    func entry(_ id: Int32) -> AtlasEntry { entries[Int(id)] }

    /// Store a rasterised glyph. `nil` means the texture is full: call `reset()` and add again.
    func add(_ raster: Raster, key: UInt64) -> Int32? {
        var entry = AtlasEntry(
            x: 0, y: 0, width: UInt16(raster.width), height: UInt16(raster.height), left: Int16(clamping: raster.left),
            top: Int16(clamping: raster.top), isColor: raster.isColor)
        if !raster.pixels.isEmpty {
            // One pixel of padding keeps the neighbours out of a sample taken at an edge.
            guard let origin = raster.isColor ? color.place(raster.width + 1, raster.height + 1) : mask.place(raster.width + 1, raster.height + 1)
            else { return nil }
            entry.x = UInt16(origin.x)
            entry.y = UInt16(origin.y)
            let region = MTLRegionMake2D(origin.x, origin.y, raster.width, raster.height)
            raster.pixels.withUnsafeBytes { bytes in
                (raster.isColor ? colorTexture : maskTexture).replace(
                    region: region, mipmapLevel: 0, withBytes: bytes.baseAddress!,
                    bytesPerRow: raster.width * (raster.isColor ? 4 : 1))
            }
        }
        rasterised += 1
        let id = Int32(entries.count)
        entries.append(entry)
        index[key] = id
        return id
    }

    func reset() {
        entries.removeAll(keepingCapacity: true)
        index.removeAll(keepingCapacity: true)
        mask = ShelfPacker(size: Self.maskSize)
        color = ShelfPacker(size: Self.colorSize)
        resets += 1
        generation += 1
    }
}

/// Rows of equal-ish height filled left to right: simple, and good enough for glyphs of one size.
private struct ShelfPacker {
    private let size: Int
    private var shelves: [(y: Int, height: Int, x: Int)] = []
    private var nextY = 0

    init(size: Int) { self.size = size }

    mutating func place(_ width: Int, _ height: Int) -> (x: Int, y: Int)? {
        guard width <= size, height <= size else { return nil }
        for index in shelves.indices where shelves[index].height >= height && shelves[index].height <= height + height / 2 + 2 {
            if shelves[index].x + width <= size {
                let origin = (shelves[index].x, shelves[index].y)
                shelves[index].x += width
                return origin
            }
        }
        guard nextY + height <= size else { return nil }
        shelves.append((nextY, height, width))
        defer { nextY += height }
        return (0, nextY)
    }
}
