// A rendered frame read back from the GPU, and how it becomes a PNG.

import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

public struct RenderedImage: Sendable {
    public var width: Int
    public var height: Int
    /// Four bytes a pixel, blue first, top row first, opaque.
    public var bgra: [UInt8]

    public func pixel(_ x: Int, _ y: Int) -> (r: UInt8, g: UInt8, b: UInt8) {
        let offset = (y * width + x) * 4
        return (bgra[offset + 2], bgra[offset + 1], bgra[offset])
    }

    public func writePNG(to url: URL) throws {
        let image: CGImage? = bgra.withUnsafeBytes { raw in
            guard
                let provider = CGDataProvider(data: Data(raw) as CFData),
                let space = CGColorSpace(name: CGColorSpace.sRGB)
            else { return nil }
            return CGImage(
                width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4, space: space,
                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue),
                provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
        }
        guard let image, let destination = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else {
            throw CocoaError(.fileWriteUnknown)
        }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination) else { throw CocoaError(.fileWriteUnknown) }
    }
}
