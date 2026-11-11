// The compiled shader and a queue to draw with: built once per intro (a few milliseconds). `encode` draws one
// frame into a texture; the overlay presents it, a test reads it back (`pixels`).

import AuxModel
import Foundation
import Metal

final class IntroRenderer {
    static let pixelFormat = MTLPixelFormat.bgra8Unorm

    enum SetupError: Error { case noDevice, noQueue, noFunction(String) }

    let device: MTLDevice
    private let queue: MTLCommandQueue
    private let pipeline: MTLRenderPipelineState

    init() throws {
        guard let device = MTLCreateSystemDefaultDevice() else { throw SetupError.noDevice }
        self.device = device
        let library = try device.makeLibrary(source: try IntroShader.source(), options: nil)
        guard let vertex = library.makeFunction(name: "intro_vertex") else { throw SetupError.noFunction("intro_vertex") }
        guard let fragment = library.makeFunction(name: "intro_fragment") else {
            throw SetupError.noFunction("intro_fragment")
        }
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = vertex
        descriptor.fragmentFunction = fragment
        descriptor.colorAttachments[0].pixelFormat = Self.pixelFormat
        pipeline = try device.makeRenderPipelineState(descriptor: descriptor)
        guard let queue = device.makeCommandQueue() else { throw SetupError.noQueue }
        self.queue = queue
    }

    /// Encodes one frame of `uniforms` into `texture`, cleared to transparent; the caller presents or reads it.
    func encode(into texture: MTLTexture, uniforms: IntroUniforms, trail: [SIMD2<Float>]) -> MTLCommandBuffer? {
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 0)
        pass.colorAttachments[0].storeAction = .store
        guard let buffer = queue.makeCommandBuffer(), let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else {
            return nil
        }
        encoder.setRenderPipelineState(pipeline)
        var uniforms = uniforms
        encoder.setFragmentBytes(&uniforms, length: MemoryLayout<IntroUniforms>.stride, index: 0)
        var trail = trail
        // The shader reads a fixed 24 positions; a short list is padded rather than read past its end.
        trail += Array(repeating: trail.last ?? .zero, count: max(0, Voyage.trailPoints - trail.count))
        encoder.setFragmentBytes(&trail, length: MemoryLayout<SIMD2<Float>>.stride * Voyage.trailPoints, index: 1)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        return buffer
    }

    /// One frame drawn off screen: `width` x `height` pixels, premultiplied RGBA, rows top first.
    func pixels(width: Int, height: Int, uniforms: IntroUniforms, trail: [SIMD2<Float>]) -> [UInt8]? {
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: Self.pixelFormat, width: width, height: height, mipmapped: false)
        descriptor.usage = .renderTarget
        descriptor.storageMode = .managed
        guard let texture = device.makeTexture(descriptor: descriptor),
            let buffer = encode(into: texture, uniforms: uniforms, trail: trail),
            let blit = buffer.makeBlitCommandEncoder()
        else { return nil }
        blit.synchronize(resource: texture)
        blit.endEncoding()
        buffer.commit()
        buffer.waitUntilCompleted()
        var bgra = [UInt8](repeating: 0, count: width * height * 4)
        texture.getBytes(&bgra, bytesPerRow: width * 4, from: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0)
        for index in stride(from: 0, to: bgra.count, by: 4) { bgra.swapAt(index, index + 2) }
        return bgra
    }
}
