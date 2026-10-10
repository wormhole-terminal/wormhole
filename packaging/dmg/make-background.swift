#!/usr/bin/env swift
// Draws the disk image's background: background.png (660 x 400 pt) and background@2x.png (the same at 1320 x 800
// px), which scripts/make-dmg.sh joins into the Retina TIFF Finder shows behind the icons.
//
// The picture is plain white with an arrow between the two icons and a line of instruction. White, not
// transparent: a window with any background picture is drawn as a light-mode window, and Finder keeps the icon
// names black (it never switches them for dark mode), so an opaque light picture makes the window look the same in
// both appearances and keeps the names readable. A transparent one leaves it to whatever Finder shows underneath.
// The arrow and the text are drawn in colors that read on white.
//
// The two icons sit at the points below (scripts/make-dmg.sh puts them there), so the arrow runs between them.
//
//   swift packaging/dmg/make-background.swift packaging/dmg        # rewrites the two PNGs there
import AppKit

let width: CGFloat = 660, height: CGFloat = 400
let appCenter = CGPoint(x: 165, y: 185), applicationsCenter = CGPoint(x: 495, y: 185)

func rgb(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(
        srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
        blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
}

func render(scale: CGFloat, to url: URL) throws {
    let w = Int(width * scale), h = Int(height * scale)
    guard
        let ctx = CGContext(
            data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
            space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { exit(1) }
    // Top-left origin, in points.
    ctx.translateBy(x: 0, y: CGFloat(h))
    ctx.scaleBy(x: scale, y: -scale)

    ctx.setFillColor(rgb(0xFFFFFF))
    ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))

    // The arrow between the two icons: three chevrons in the app's blue, fading toward the app.
    ctx.setLineCap(.round)
    ctx.setLineJoin(.round)
    ctx.setLineWidth(7)
    let y = appCenter.y
    let midX = (appCenter.x + applicationsCenter.x) / 2
    for (i, alpha) in [(-1, 0.4), (0, 0.7), (1, 1.0)] as [(Int, CGFloat)] {
        let x = midX + CGFloat(i) * 30
        ctx.setStrokeColor(rgb(0x4F6BFF, alpha))
        ctx.move(to: CGPoint(x: x - 9, y: y - 20))
        ctx.addLine(to: CGPoint(x: x + 9, y: y))
        ctx.addLine(to: CGPoint(x: x - 9, y: y + 20))
        ctx.strokePath()
    }

    // The instruction, at the bottom, clear of Finder's path bar when it is shown.
    let flipped = NSGraphicsContext(cgContext: ctx, flipped: true)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = flipped
    let style = NSMutableParagraphStyle()
    style.alignment = .center
    let text = NSAttributedString(
        string: "Drag Wormhole into your Applications folder",
        attributes: [
            .font: NSFont.systemFont(ofSize: 14, weight: .semibold),
            .foregroundColor: NSColor(cgColor: rgb(0x7B7F8F))!,
            .paragraphStyle: style,
        ])
    text.draw(in: CGRect(x: 0, y: 322, width: width, height: 24))
    NSGraphicsContext.restoreGraphicsState()

    guard let image = ctx.makeImage(),
        let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil)
    else { exit(1) }
    CGImageDestinationAddImage(dest, image, nil)
    guard CGImageDestinationFinalize(dest) else { exit(1) }
}

guard CommandLine.arguments.count == 2 else {
    FileHandle.standardError.write(Data("usage: make-background.swift <out dir>\n".utf8))
    exit(2)
}
let out = URL(fileURLWithPath: CommandLine.arguments[1])
try render(scale: 1, to: out.appendingPathComponent("background.png"))
try render(scale: 2, to: out.appendingPathComponent("background@2x.png"))
