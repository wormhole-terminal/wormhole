// The pictures as textures, made once per key and kept under a budget. A key is never reused for other pixels
// (the engine counts handles up and never recycles one), so a texture is valid for as long as it is held.
// Mip levels are generated on the GPU in the frame that first draws a picture, so a big screenshot shrunk to a
// few cells is not aliased.

import Metal
import TerminalCore

@MainActor
final class ImageTextures {
    /// What the texture cache may hold in all: the engine's own per-terminal budget for decoded images
    /// (`wormhole-term` `images/store.rs` `STORE_BUDGET`), so a picture the engine keeps is one this keeps.
    static let budgetBytes = 128 * 1024 * 1024

    private struct Entry {
        var texture: MTLTexture
        var bytes: Int
        var used: UInt64
    }

    private let device: MTLDevice
    private let budget: Int
    private var entries: [UInt32: Entry] = [:]
    private var tick: UInt64 = 0
    private var total = 0
    /// Textures that still need their mip levels, done in the next command buffer.
    private var needMips: [MTLTexture] = []

    init(device: MTLDevice, budget: Int = ImageTextures.budgetBytes) {
        self.device = device
        self.budget = budget
    }

    var count: Int { entries.count }
    var bytes: Int { total }
    func has(_ key: UInt32) -> Bool { entries[key] != nil }

    /// The texture for `key`, marked as used now.
    func texture(for key: UInt32) -> MTLTexture? {
        tick += 1
        entries[key]?.used = tick
        return entries[key]?.texture
    }

    /// Make the texture for `image`; false when the pixels do not match the size or Metal cannot make one.
    @discardableResult
    func insert(_ image: InlineImage, key: UInt32) -> Bool {
        guard image.width > 0, image.height > 0, image.rgba.count == image.width * image.height * 4 else { return false }
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .rgba8Unorm, width: image.width, height: image.height, mipmapped: true)
        descriptor.usage = .shaderRead
        guard let texture = device.makeTexture(descriptor: descriptor) else { return false }
        image.rgba.withUnsafeBytes {
            texture.replace(
                region: MTLRegionMake2D(0, 0, image.width, image.height), mipmapLevel: 0, withBytes: $0.baseAddress!,
                bytesPerRow: image.width * 4)
        }
        let size = image.rgba.count
        remove(key)
        tick += 1
        entries[key] = Entry(texture: texture, bytes: size, used: tick)
        total += size
        needMips.append(texture)
        return true
    }

    /// Drop the least recently used textures, other than those of `keep`, until the budget holds.
    func trim(keeping keep: Set<UInt32>) {
        while total > budget,
            let oldest = entries.filter({ !keep.contains($0.key) }).min(by: { $0.value.used < $1.value.used })?.key
        {
            remove(oldest)
        }
    }

    private func remove(_ key: UInt32) {
        guard let old = entries.removeValue(forKey: key) else { return }
        total -= old.bytes
        needMips.removeAll { $0 === old.texture }
    }

    /// Generate the mip levels of the textures made since the last frame, before the pass samples them.
    func prepare(in buffer: MTLCommandBuffer) {
        guard !needMips.isEmpty, let blit = buffer.makeBlitCommandEncoder() else { return }
        for texture in needMips { blit.generateMipmaps(for: texture) }
        blit.endEncoding()
        needMips.removeAll()
    }
}
