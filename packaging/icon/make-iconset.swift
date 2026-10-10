#!/usr/bin/env swift
// Lays Icon Composer's render of the icon out as a macOS .iconset: the artwork on the standard 824 pt
// tile inside the 1024 pt canvas, with the soft shadow icons of that era carry, at every size iconutil
// wants. build-icns.sh runs it and turns the folder into AppIcon.icns.
//
//   make-iconset.swift <render.png, 1024 px, full bleed> <out.iconset>
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
guard args.count == 3,
    let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil),
    let art = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    FileHandle.standardError.write(Data("usage: make-iconset.swift <render.png> <out.iconset>\n".utf8))
    exit(2)
}
let out = URL(fileURLWithPath: args[2])
try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

// iconutil's names and the pixel size each one holds.
let sizes: [(String, Int)] = [
    ("icon_16x16", 16), ("icon_16x16@2x", 32), ("icon_32x32", 32), ("icon_32x32@2x", 64),
    ("icon_128x128", 128), ("icon_128x128@2x", 256), ("icon_256x256", 256), ("icon_256x256@2x", 512),
    ("icon_512x512", 512), ("icon_512x512@2x", 1024),
]

for (name, px) in sizes {
    let s = CGFloat(px) / 1024
    guard
        let ctx = CGContext(
            data: nil, width: px, height: px, bitsPerComponent: 8, bytesPerRow: 0,
            space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { exit(1) }
    ctx.interpolationQuality = .high
    // The 824 pt tile on the 1024 pt grid, a little above centre so the shadow has room underneath.
    ctx.setShadow(offset: CGSize(width: 0, height: -10 * s), blur: 20 * s, color: CGColor(gray: 0, alpha: 0.3))
    ctx.draw(art, in: CGRect(x: 100 * s, y: 100 * s, width: 824 * s, height: 824 * s))
    guard let image = ctx.makeImage(),
        let dest = CGImageDestinationCreateWithURL(
            out.appendingPathComponent("\(name).png") as CFURL, UTType.png.identifier as CFString, 1, nil)
    else { exit(1) }
    CGImageDestinationAddImage(dest, image, nil)
    guard CGImageDestinationFinalize(dest) else { exit(1) }
}
