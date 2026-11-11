// Texture readback of Metal output without Screen Recording permission: an offscreen render target and
// a CAMetalLayer drawable (framebufferOnly = false), each read with getBytes and written as a PNG.
// Run: swiftc -O apple/tools/metal-readback.swift -o /tmp/readback && /tmp/readback (from a scratch dir).
import Metal
import QuartzCore
import ImageIO
import UniformTypeIdentifiers
import Foundation

let device = MTLCreateSystemDefaultDevice()!
let queue = device.makeCommandQueue()!
let w = 64, h = 32

func clear(_ texture: MTLTexture, _ color: MTLClearColor) {
    let pass = MTLRenderPassDescriptor()
    pass.colorAttachments[0].texture = texture
    pass.colorAttachments[0].loadAction = .clear
    pass.colorAttachments[0].storeAction = .store
    pass.colorAttachments[0].clearColor = color
    let cb = queue.makeCommandBuffer()!
    cb.makeRenderCommandEncoder(descriptor: pass)!.endEncoding()
    cb.commit()
    cb.waitUntilCompleted()
}

func firstPixel(_ texture: MTLTexture) -> [UInt8] {
    var bytes = [UInt8](repeating: 0, count: w * h * 4)
    texture.getBytes(&bytes, bytesPerRow: w * 4, from: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0)
    return Array(bytes[0..<4])
}

// 1. an offscreen texture, shared storage, like a `shot` command would render into
let desc = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .bgra8Unorm, width: w, height: h, mipmapped: false)
desc.usage = [.renderTarget, .shaderRead]
desc.storageMode = .shared
let offscreen = device.makeTexture(descriptor: desc)!
clear(offscreen, MTLClearColor(red: 1, green: 0, blue: 0, alpha: 1))
print("offscreen BGRA pixel 0:", firstPixel(offscreen), "(expect [0, 0, 255, 255])")

// 2. a drawable of a CAMetalLayer that is not in any window
let layer = CAMetalLayer()
layer.device = device
layer.pixelFormat = .bgra8Unorm
layer.framebufferOnly = false
layer.drawableSize = CGSize(width: w, height: h)
if let drawable = layer.nextDrawable() {
    clear(drawable.texture, MTLClearColor(red: 0, green: 1, blue: 0, alpha: 1))
    // getBytes on a drawable's (private-storage on some GPUs) texture: works only when it is not private.
    print("drawable storage mode:", drawable.texture.storageMode.rawValue, "(0 shared, 1 managed, 2 private)")
    if drawable.texture.storageMode != .private {
        print("drawable BGRA pixel 0:", firstPixel(drawable.texture), "(expect [0, 255, 0, 255])")
    }
} else {
    print("no drawable without a window")
}

// 3. a PNG through ImageIO
var bytes = [UInt8](repeating: 0, count: w * h * 4)
offscreen.getBytes(&bytes, bytesPerRow: w * 4, from: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0)
let cs = CGColorSpaceCreateDeviceRGB()
let info = CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue
let image = bytes.withUnsafeMutableBytes { raw in
    CGContext(data: raw.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: cs, bitmapInfo: info)!.makeImage()!
}
let url = URL(fileURLWithPath: "readback.png")
let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(dest, image, nil)
print("png written:", CGImageDestinationFinalize(dest), (try? FileManager.default.attributesOfItem(atPath: "readback.png")[.size]) ?? "?")
