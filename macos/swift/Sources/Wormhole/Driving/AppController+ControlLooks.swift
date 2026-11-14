// text-file: a test driver's replies on the control folder, not interface text
// What a window's views drew, read from the views themselves rather than from the model they were given, for the
// end-to-end tests (`macos/parity/DRIVING.md`):
//   `colour-count <#rrggbb> [x y w h] [click]`  how many pixels of the window (the one `pointer-window` names, else the
//                                               workspace's), as its views draw it now, are that colour, and the box they
//                                               fill; with `click`, a click at the box's centre, as `click` makes one
//   `marked [id]`                               the design's elements laid out in that window (inspect mode on: each one's
//                                               probe), with their frames: `marked <id> x y w h` from the content's top left
// Points are the window content's, from its top left. A colour is matched in sRGB within a few steps of each channel, so
// what the display's colour space and antialiasing do to an edge is not counted, and a fill is.

import AppKit
import DesignSystem

extension AppController {
    /// False when `command` is not one of these.
    func runLooks(command: String, argument: String) async -> Bool {
        guard command == "colour-count" || command == "marked" else { return false }
        guard let target = drivenWindow ?? window, let content = target.contentView else { reply("error: no window"); return true }
        let words = argument.split(separator: " ").map(String.init)
        if command == "marked" {
            let lines = Inspection.shared.elements(in: target)
                .filter { words.isEmpty || $0.site.id == words[0] }
                .map { (id: $0.site.id, rect: Self.topLeft(content.convert($0.frame, from: nil), in: content)) }
                .sorted { ($0.rect.minY, $0.rect.minX) < ($1.rect.minY, $1.rect.minX) }
                .map { "marked \($0.id) \(Int($0.rect.minX)) \(Int($0.rect.minY)) \(Int($0.rect.width)) \(Int($0.rect.height))" }
            reply((["marked n=\(lines.count)\(Inspection.shared.isOn ? "" : " (inspect mode is off: inspect on)")"] + lines).joined(separator: "\n"))
            return true
        }
        guard let wanted = words.first.flatMap(Self.rgb) else { reply("error: colour-count <#rrggbb> [x y w h] [click]"); return true }
        let numbers = words.dropFirst().compactMap(Double.init)
        let region = numbers.count >= 4 ? CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3]) : content.bounds
        content.layoutSubtreeIfNeeded()
        guard let bitmap = content.bitmapImageRepForCachingDisplay(in: content.bounds) else { reply("error: the window drew nothing"); return true }
        content.cacheDisplay(in: content.bounds, to: bitmap)
        guard let image = bitmap.cgImage, let (count, box) = Self.count(wanted, in: image, region: region, points: content.bounds.size) else {
            reply("error: the window's picture could not be read")
            return true
        }
        var said = "colour-count \(words[0]) pixels=\(count) box=" + (box.map { "\(Int($0.minX)),\(Int($0.minY)),\(Int($0.width)),\(Int($0.height))" } ?? "-")
        if words.contains("click"), let box {
            let point = injector.location(Double(box.midX), Double(box.midY), in: target)
            said += " " + injector.press(point, in: target, clicks: 1, flags: [])
            if injector.isPressed { said += ", " + injector.release(point, in: target, clicks: 1, flags: []) }
            // SwiftUI acts on the release on the next turn of the run loop.
            await pause(60)
        }
        reply(said)
        return true
    }

    /// A frame in the content view's own space, from its top left.
    private static func topLeft(_ rect: CGRect, in content: NSView) -> CGRect {
        content.isFlipped ? rect : CGRect(x: rect.minX, y: content.bounds.height - rect.maxY, width: rect.width, height: rect.height)
    }

    private static func rgb(_ hex: String) -> (Int, Int, Int)? {
        let digits = hex.hasPrefix("#") ? hex.dropFirst() : Substring(hex)
        guard digits.count == 6, let value = Int(digits, radix: 16) else { return nil }
        return ((value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff)
    }

    /// The pixels of `region` (points, from the top left) that are `wanted` once drawn into sRGB, and the box round them in points.
    private static func count(_ wanted: (Int, Int, Int), in image: CGImage, region: CGRect, points: CGSize) -> (Int, CGRect?)? {
        let width = image.width, height = image.height
        guard points.width > 0, let space = CGColorSpace(name: CGColorSpace.sRGB),
            let context = CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4, space: space,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        guard let data = context.data else { return nil }
        let bytes = data.bindMemory(to: UInt8.self, capacity: width * height * 4)
        let scale = Double(width) / Double(points.width)
        let x0 = max(0, Int(region.minX * scale)), x1 = min(width, Int(region.maxX * scale))
        let y0 = max(0, Int(region.minY * scale)), y1 = min(height, Int(region.maxY * scale))
        var count = 0, minX = Int.max, minY = Int.max, maxX = -1, maxY = -1
        // The context's rows run from the top of the picture, as `region`'s do.
        for y in y0..<max(y0, y1) {
            for x in x0..<max(x0, x1) {
                let at = (y * width + x) * 4
                guard bytes[at + 3] == 255, abs(Int(bytes[at]) - wanted.0) <= 4, abs(Int(bytes[at + 1]) - wanted.1) <= 4,
                    abs(Int(bytes[at + 2]) - wanted.2) <= 4
                else { continue }
                count += 1
                (minX, minY, maxX, maxY) = (min(minX, x), min(minY, y), max(maxX, x), max(maxY, y))
            }
        }
        let box = count == 0 ? nil : CGRect(
            x: Double(minX) / scale, y: Double(minY) / scale, width: Double(maxX - minX + 1) / scale, height: Double(maxY - minY + 1) / scale)
        return (count, box)
    }
}
