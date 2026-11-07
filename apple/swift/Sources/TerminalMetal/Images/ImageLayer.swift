// The pictures of one terminal view: it keeps the placements the frames carried, asks the source for each picture's
// pixels once (a picture still being decoded is asked for again on the next frame; the engine wakes the view when
// the decode is done), and draws them in three bands as kitty's protocol stacks them by `z`: below the cell
// backgrounds (z at or under -1073741824), between the backgrounds and the text (any other negative z), and over
// everything (z of 0 and up). The renderer calls `apply` with each frame, `prepare` and `encode` while drawing.

import Metal
import TerminalCore

@MainActor
final class ImageLayer {
    enum Band { case belowBackgrounds, belowText, aboveText }

    /// kitty: images with z under this go below the cells' background colours.
    static let belowBackgroundsZ: Int32 = -1_073_741_824

    /// Asks for a picture's pixels; the view points it at its source.
    var provider: ((UInt32) -> InlineImage?)?

    private let pipeline: MTLRenderPipelineState
    let textures: ImageTextures
    private(set) var placements: [ImagePlacement] = []
    /// Rows the view is scrolled into the history, as the last frame said.
    private(set) var displayOffset = 0
    private var missing: Set<UInt32> = []

    init(device: MTLDevice, pixelFormat: MTLPixelFormat = .bgra8Unorm) throws {
        textures = ImageTextures(device: device)
        let library: MTLLibrary
        do { library = try device.makeLibrary(source: ImageShaders.source, options: nil) } catch { throw RendererError.pipeline("\(error)") }
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = library.makeFunction(name: "image_vertex")
        descriptor.fragmentFunction = library.makeFunction(name: "image_fragment")
        let attachment = descriptor.colorAttachments[0]!
        attachment.pixelFormat = pixelFormat
        attachment.isBlendingEnabled = true
        attachment.sourceRGBBlendFactor = .one
        attachment.sourceAlphaBlendFactor = .one
        attachment.destinationRGBBlendFactor = .oneMinusSourceAlpha
        attachment.destinationAlphaBlendFactor = .oneMinusSourceAlpha
        do { pipeline = try device.makeRenderPipelineState(descriptor: descriptor) } catch { throw RendererError.pipeline("\(error)") }
    }

    /// Take what a frame says about the pictures. Returns whether anything to draw may have changed.
    func apply(_ data: Data) throws -> Bool {
        guard !data.isEmpty else { return false }
        var changed = false
        try data.withUnsafeBytes { bytes in
            let header = try FrameReader.header(bytes)
            if header.displayOffset != displayOffset {
                displayOffset = header.displayOffset
                changed = !placements.isEmpty
            }
            if let next = try PlacementRecords.read(bytes, header: header), next != placements {
                placements = next
                changed = true
            }
        }
        if fetchMissing() { changed = true }
        return changed
    }

    /// Ask for the pixels of every placed picture that has no texture yet.
    private func fetchMissing() -> Bool {
        var fetched = false
        let wanted = Set(placements.map(\.key))
        missing = []
        for key in wanted where !textures.has(key) {
            if let image = provider?(key), textures.insert(image, key: key) { fetched = true } else { missing.insert(key) }
        }
        textures.trim(keeping: wanted)
        return fetched
    }

    /// Whether some placed picture is still waiting for its pixels.
    var isWaiting: Bool { !missing.isEmpty }

    func prepare(in buffer: MTLCommandBuffer) { textures.prepare(in: buffer) }

    /// Draw the placements of `band` into the pass of a surface of `size` pixels; the grid fills `grid` of it.
    func encode(_ band: Band, into encoder: MTLRenderCommandEncoder, size: SIMD2<Float>, surface: ImageSurface) {
        var drawing = false
        for placement in placements.sorted(by: { $0.z < $1.z }) where Self.band(of: placement.z) == band {
            guard let texture = textures.texture(for: placement.key),
                var quad = ImageLayout.quad(
                    placement, imageWidth: texture.width, imageHeight: texture.height, displayOffset: displayOffset,
                    surface: surface)
            else { continue }
            if !drawing {
                encoder.setRenderPipelineState(pipeline)
                var viewport = size
                encoder.setVertexBytes(&viewport, length: MemoryLayout<SIMD2<Float>>.size, index: 4)
                drawing = true
            }
            encoder.setVertexBytes(&quad, length: MemoryLayout<ImageQuad>.size, index: 3)
            encoder.setFragmentTexture(texture, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        }
    }

    static func band(of z: Int32) -> Band {
        z <= belowBackgroundsZ ? .belowBackgrounds : z < 0 ? .belowText : .aboveText
    }
}
