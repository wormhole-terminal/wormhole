// The Metal side: pipelines, a ring of instance buffers, and one render pass in three layers
// (backgrounds, glyphs, decorations). It draws what `TerminalScene` holds; it never reads a frame.
//
// Up to three frames are in flight. The CPU never waits for the GPU: if all three are busy the draw is
// skipped (`.busy`) and the view tries again on its next tick, so a slow GPU cannot stall the main thread.

import AppKit
import Metal
import QuartzCore
import Synchronization
import TerminalCore

public struct RendererConfiguration: Sendable {
    public var font: FontSpec
    public var pointSize: Double
    /// A multiple of the font size; `nil` takes the font's own line.
    public var lineHeight: Double?
    /// Device pixels per point.
    public var scale: Double
    public var palette: TerminalPalette

    public init(
        font: FontSpec = .system(family: "Menlo"), pointSize: Double = 13, lineHeight: Double? = nil, scale: Double = 2,
        palette: TerminalPalette = .spike
    ) {
        self.font = font
        self.pointSize = pointSize
        self.lineHeight = lineHeight
        self.scale = scale
        self.palette = palette
    }
}

public enum RendererError: Error {
    case noDevice
    case pipeline(String)
}

/// How long a draw spent where. `assemble` is filling the GPU buffers, `encode` the command buffer's
/// creation, encoding and commit, `wait` blocking for a free frame or a drawable (not CPU work).
public struct DrawTimings: Sendable {
    public var outcome: DrawOutcome
    public var waitNs: UInt64 = 0
    public var assembleNs: UInt64 = 0
    public var encodeNs: UInt64 = 0
}

public enum DrawOutcome: Sendable, Equatable {
    case drawn
    /// Three frames are still on the GPU: try again next tick.
    case busy
    case noDrawable
}

/// The GPU's own time per frame, filled in from command-buffer completion handlers on Metal's threads.
public final class GPUTimes: Sendable {
    private let samples = Mutex<[Double]>([])
    func record(_ seconds: Double) { samples.withLock { $0.append(seconds) } }
    public func take() -> [Double] { samples.withLock { let all = $0; $0 = []; return all } }
}

@MainActor
public final class MetalRenderer {
    private struct Uniforms {
        var viewport: SIMD2<Float>
        var dim: Float = 0.66
        var pad: Float = 0
    }

    private struct Slot {
        var solids: MTLBuffer
        var glyphs: MTLBuffer
    }

    public let device: MTLDevice
    let scene: TerminalScene
    private let queue: MTLCommandQueue
    private let solidPipeline: MTLRenderPipelineState
    private let glyphPipeline: MTLRenderPipelineState
    private var slots: [Slot]
    private var nextSlot = 0
    private let inflight = DispatchSemaphore(value: 3)
    public let gpuTimes = GPUTimes()
    public private(set) var palette: TerminalPalette
    public private(set) var configuration: RendererConfiguration

    public init(configuration: RendererConfiguration, device: MTLDevice? = MTLCreateSystemDefaultDevice()) throws {
        guard let device, let queue = device.makeCommandQueue() else { throw RendererError.noDevice }
        self.device = device
        self.queue = queue
        self.configuration = configuration
        palette = configuration.palette
        scene = TerminalScene(
            fonts: FontSet(
                spec: configuration.font, pointSize: CGFloat(configuration.pointSize), scale: CGFloat(configuration.scale),
                lineHeight: configuration.lineHeight.map { CGFloat($0) }),
            device: device)

        let library: MTLLibrary
        do { library = try device.makeLibrary(source: Shaders.source, options: nil) } catch { throw RendererError.pipeline("\(error)") }
        func pipeline(_ vertex: String, _ fragment: String) throws -> MTLRenderPipelineState {
            let descriptor = MTLRenderPipelineDescriptor()
            descriptor.vertexFunction = library.makeFunction(name: vertex)
            descriptor.fragmentFunction = library.makeFunction(name: fragment)
            let attachment = descriptor.colorAttachments[0]!
            attachment.pixelFormat = .bgra8Unorm
            // Premultiplied alpha: the fragment returns colour already multiplied by coverage.
            attachment.isBlendingEnabled = true
            attachment.sourceRGBBlendFactor = .one
            attachment.sourceAlphaBlendFactor = .one
            attachment.destinationRGBBlendFactor = .oneMinusSourceAlpha
            attachment.destinationAlphaBlendFactor = .oneMinusSourceAlpha
            do { return try device.makeRenderPipelineState(descriptor: descriptor) } catch { throw RendererError.pipeline("\(error)") }
        }
        solidPipeline = try pipeline("solid_vertex", "solid_fragment")
        glyphPipeline = try pipeline("glyph_vertex", "glyph_fragment")
        slots = (0..<3).map { _ in
            Slot(
                solids: device.makeBuffer(length: 64 * 1024, options: [.storageModeShared, .cpuCacheModeWriteCombined])!,
                glyphs: device.makeBuffer(length: 64 * 1024, options: [.storageModeShared, .cpuCacheModeWriteCombined])!)
        }
        scene.setKeepsTextInSelection(configuration.palette.keepsTextInSelection)
    }

    // MARK: geometry

    /// One cell in device pixels.
    public var cellPixels: (width: Int, height: Int) { (scene.fonts.metrics.width, scene.fonts.metrics.height) }
    public var scale: Double { configuration.scale }

    /// The cells that fit a surface of this many pixels.
    public func grid(fittingPixels width: Int, _ height: Int) -> (cols: Int, rows: Int) {
        let cell = cellPixels
        return (max(1, width / cell.width), max(1, height / cell.height))
    }

    /// The size in points of a view that holds exactly this grid.
    public func pointSize(forGrid cols: Int, _ rows: Int) -> CGSize {
        CGSize(width: Double(cols * cellPixels.width) / scale, height: Double(rows * cellPixels.height) / scale)
    }

    /// The pixels the grid itself covers.
    public var gridPixels: (width: Int, height: Int) { (scene.columns * cellPixels.width, scene.rows * cellPixels.height) }

    public func setPalette(_ value: TerminalPalette) {
        palette = value
        scene.setKeepsTextInSelection(value.keepsTextInSelection)
        scene.rebuildDirty()
        scene.touch()
    }

    /// The selected columns per row of the frame last applied, and the link under the pointer.
    public func setSelection(_ spans: [RowSpan]) {
        scene.setSelection(spans)
        scene.rebuildDirty()
    }

    public func setLinkHighlight(_ spans: [RowSpan]) {
        scene.setLink(spans)
        scene.rebuildDirty()
    }

    // MARK: frames

    /// Take a packed frame; the rows it changed are rebuilt. Returns the number rebuilt.
    @discardableResult
    public func apply(frame: Data) throws -> Int { try scene.apply(frame) }

    /// The instances changed since the last draw.
    public var needsDraw: Bool { scene.changeCount != drawnChange }
    private var drawnChange = -1

    public func setCursorBlinkOn(_ on: Bool) { scene.setCursorOn(on); scene.rebuildDirty() }
    public func setFocused(_ focused: Bool) { scene.setFocused(focused); scene.rebuildDirty() }
    func setMarked(_ marked: MarkedText?) {
        scene.setMarked(marked)
        scene.rebuildDirty()
    }

    /// Counters for the measuring mode (words shaped, glyphs rasterised, rows rebuilt since the start).
    public var counters: RendererCounters {
        RendererCounters(
            frames: scene.counters.frames, rowsRebuilt: scene.counters.rowsRebuilt, cellsRead: scene.counters.cellsRead,
            wordsShaped: scene.words.shaped, wordHits: scene.words.hits, glyphsRasterised: scene.atlas.rasterised,
            atlasResets: scene.atlas.resets)
    }

    /// The cursor's cell, if shown, for the input method's candidate window.
    public var cursorCell: (row: Int, column: Int)? { scene.cursor.map { ($0.row, $0.column) } }
    public var gridSize: (cols: Int, rows: Int) { (scene.columns, scene.rows) }
    public func text(ofRow row: Int) -> String { scene.text(ofRow: row) }

    // MARK: drawing

    /// Draw the scene into the layer's next drawable. With `blocking` off it never waits for the GPU: all
    /// three frames in flight means `.busy`, and the view tries again on its next tick. The measuring mode
    /// blocks, since a forced draw must happen, and reports the wait apart from the CPU work.
    public func draw(in layer: CAMetalLayer, blocking: Bool = false) -> DrawTimings {
        var timings = DrawTimings(outcome: .drawn)
        let t0 = MonotonicClock.now()
        if blocking {
            inflight.wait()
        } else if inflight.wait(timeout: .now()) != .success {
            timings.outcome = .busy
            return timings
        }
        let t1 = MonotonicClock.now()
        let slot = nextSlot
        nextSlot = (nextSlot + 1) % slots.count
        let counts = assemble(slot: slot)
        let t2 = MonotonicClock.now()
        guard let drawable = layer.nextDrawable() else {
            inflight.signal()
            timings.outcome = .noDrawable
            return timings
        }
        let t3 = MonotonicClock.now()
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].storeAction = .store
        pass.colorAttachments[0].clearColor = clearColor
        let buffer = queue.makeCommandBuffer()!
        encode(into: buffer, pass: pass, size: layer.drawableSize, slot: slot, counts: counts)
        buffer.present(drawable)
        let semaphore = inflight
        let times = gpuTimes
        buffer.addCompletedHandler { finished in
            times.record(finished.gpuEndTime - finished.gpuStartTime)
            semaphore.signal()
        }
        buffer.commit()
        let t4 = MonotonicClock.now()
        drawnChange = scene.changeCount
        timings.waitNs = (t1 - t0) + (t3 - t2)
        timings.assembleNs = t2 - t1
        timings.encodeNs = t4 - t3
        return timings
    }

    private var clearColor: MTLClearColor {
        let background = palette.background
        return MTLClearColor(red: Double(background.x) / 255, green: Double(background.y) / 255, blue: Double(background.z) / 255, alpha: 1)
    }

    /// Fill the slot's GPU buffers from every row's instances (growing them if need be).
    private func assemble(slot index: Int) -> (backgrounds: Int, decorations: Int, glyphs: Int) {
        let counts = scene.instanceCounts
        let solidBytes = (counts.backgrounds + counts.decorations) * MemoryLayout<SolidInstance>.stride
        let glyphBytes = counts.glyphs * MemoryLayout<GlyphInstance>.stride
        var slot = slots[index]
        if slot.solids.length < solidBytes {
            slot.solids = device.makeBuffer(length: solidBytes * 2, options: [.storageModeShared, .cpuCacheModeWriteCombined])!
        }
        if slot.glyphs.length < glyphBytes {
            slot.glyphs = device.makeBuffer(length: glyphBytes * 2, options: [.storageModeShared, .cpuCacheModeWriteCombined])!
        }
        slots[index] = slot
        scene.copyInstances(solids: slot.solids.contents(), glyphs: slot.glyphs.contents(), counts: counts)
        return counts
    }

    private func encode(
        into buffer: MTLCommandBuffer, pass: MTLRenderPassDescriptor, size: CGSize, slot index: Int,
        counts: (backgrounds: Int, decorations: Int, glyphs: Int)
    ) {
        let slot = slots[index]
        var uniforms = Uniforms(viewport: SIMD2(Float(size.width), Float(size.height)))
        let colors = palette.floats()
        let encoder = buffer.makeRenderCommandEncoder(descriptor: pass)!
        encoder.setVertexBytes(&uniforms, length: MemoryLayout<Uniforms>.size, index: 1)
        colors.withUnsafeBytes { encoder.setVertexBytes($0.baseAddress!, length: $0.count, index: 2) }

        encoder.setRenderPipelineState(solidPipeline)
        encoder.setVertexBuffer(slot.solids, offset: 0, index: 0)
        if counts.backgrounds > 0 {
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4, instanceCount: counts.backgrounds)
        }
        if counts.glyphs > 0 {
            encoder.setRenderPipelineState(glyphPipeline)
            encoder.setVertexBuffer(slot.glyphs, offset: 0, index: 0)
            encoder.setFragmentTexture(scene.atlas.maskTexture, index: 0)
            encoder.setFragmentTexture(scene.atlas.colorTexture, index: 1)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4, instanceCount: counts.glyphs)
        }
        if counts.decorations > 0 {
            encoder.setRenderPipelineState(solidPipeline)
            encoder.setVertexBuffer(slot.solids, offset: counts.backgrounds * MemoryLayout<SolidInstance>.stride, index: 0)
            encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4, instanceCount: counts.decorations)
        }
        encoder.endEncoding()
    }

    // MARK: readback

    /// Render the scene into a texture of its own and read it back: BGRA, top row first. This is what the
    /// screenshots are made from (`cacheDisplay` does not see a Metal layer), and it needs neither a window
    /// nor a drawable. Waits for the GPU.
    public func renderToImage(width: Int? = nil, height: Int? = nil) -> RenderedImage {
        let width = width ?? max(1, gridPixels.width), height = height ?? max(1, gridPixels.height)
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .bgra8Unorm, width: width, height: height, mipmapped: false)
        descriptor.usage = [.renderTarget, .shaderRead]
        descriptor.storageMode = .shared
        let texture = device.makeTexture(descriptor: descriptor)!
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].storeAction = .store
        pass.colorAttachments[0].clearColor = clearColor
        // A slot of its own would grow the ring for one call; reuse the ring's, after everything in it is done.
        for _ in 0..<3 { inflight.wait() }
        defer { for _ in 0..<3 { inflight.signal() } }
        let buffer = queue.makeCommandBuffer()!
        let counts = assemble(slot: nextSlot)
        encode(into: buffer, pass: pass, size: CGSize(width: width, height: height), slot: nextSlot, counts: counts)
        buffer.commit()
        buffer.waitUntilCompleted()
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        pixels.withUnsafeMutableBytes {
            texture.getBytes($0.baseAddress!, bytesPerRow: width * 4, from: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0)
        }
        return RenderedImage(width: width, height: height, bgra: pixels)
    }
}

public struct RendererCounters: Sendable, Equatable {
    public var frames: Int
    public var rowsRebuilt: Int
    public var cellsRead: Int
    public var wordsShaped: Int
    public var wordHits: Int
    public var glyphsRasterised: Int
    public var atlasResets: Int

    public static func - (lhs: Self, rhs: Self) -> Self {
        Self(
            frames: lhs.frames - rhs.frames, rowsRebuilt: lhs.rowsRebuilt - rhs.rowsRebuilt,
            cellsRead: lhs.cellsRead - rhs.cellsRead, wordsShaped: lhs.wordsShaped - rhs.wordsShaped,
            wordHits: lhs.wordHits - rhs.wordHits, glyphsRasterised: lhs.glyphsRasterised - rhs.glyphsRasterised,
            atlasResets: lhs.atlasResets - rhs.atlasResets)
    }
}
