import CoreGraphics
import Foundation
import ImageIO
import Testing

/// A colour as the shot holds it.
struct RGB: Hashable, CustomStringConvertible {
    var red: UInt8, green: UInt8, blue: UInt8

    init(_ red: UInt8, _ green: UInt8, _ blue: UInt8) { (self.red, self.green, self.blue) = (red, green, blue) }

    /// `#rrggbb`, the way a readout writes a colour.
    init?(hex: String) {
        guard hex.hasPrefix("#"), hex.count == 7, let value = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        self.init(UInt8(value >> 16 & 255), UInt8(value >> 8 & 255), UInt8(value & 255))
    }

    var description: String { String(format: "#%02x%02x%02x", red, green, blue) }

    /// The largest difference of a channel to `other`.
    func distance(to other: RGB) -> Int {
        max(abs(Int(red) - Int(other.red)), abs(Int(green) - Int(other.green)), abs(Int(blue) - Int(other.blue)))
    }
}

/// A `shot` of the selected terminal as the renderer drew it (the pane: the grid and its padding), read back from its texture,
/// with the cells' places in it. The cell's size and the scale come from `state` (`cell=WxHpx scale=S`), the padding is the
/// renderer's (`MetalRenderer.padding`, 4 pt).
struct GridShot {
    let width: Int
    let height: Int
    private let pixels: [UInt8]
    /// One cell's size in pixels: the width is fractional (16.8), each edge falls on the nearest pixel (`MetalRenderer.left`).
    let cell: (width: Double, height: Int)
    private let padding: Int

    /// The renderer's padding in points (`MetalRenderer.padding`).
    private static let paddingPoints = 4

    init?(png: URL, state: String) {
        guard let cellText = Instance.value("cell", in: state)?.replacingOccurrences(of: "px", with: ""),
            let scale = Instance.value("scale", in: state).flatMap({ Double($0) }),
            case let sides = cellText.split(separator: "x").compactMap({ Double($0) }), sides.count == 2,
            let source = CGImageSourceCreateWithURL(png as CFURL, nil), let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
            let space = CGColorSpace(name: CGColorSpace.sRGB)
        else { return nil }
        let width = image.width, height = image.height
        var bytes = [UInt8](repeating: 0, count: width * height * 4)
        let drawn = bytes.withUnsafeMutableBytes { raw -> Bool in
            guard let context = CGContext(
                data: raw.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4, space: space,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
            else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard drawn else { return nil }
        (self.width, self.height) = (width, height)
        pixels = bytes
        cell = (sides[0], Int(sides[1]))
        padding = Int((Double(Self.paddingPoints) * scale).rounded())
    }

    func pixel(x: Int, y: Int) -> RGB {
        let offset = (y * width + x) * 4
        return RGB(pixels[offset], pixels[offset + 1], pixels[offset + 2])
    }

    /// Column `column`'s left edge in the picture.
    func left(_ column: Int) -> Int { padding + Int((Double(column) * cell.width).rounded()) }

    /// How many pixels `columns` columns from `column` are wide.
    func width(of columns: Int, from column: Int = 0) -> Int { left(column + columns) - left(column) }

    /// The pixel `dx`, `dy` from the top left of a cell.
    func pixel(row: Int, column: Int, dx: Int = 1, dy: Int = 1) -> RGB {
        pixel(x: left(column) + dx, y: padding + row * cell.height + dy)
    }

    /// How many pixels of each colour the cell has.
    func histogram(row: Int, column: Int, columns: Int = 1) -> [RGB: Int] {
        var counts: [RGB: Int] = [:]
        for y in 0..<cell.height {
            for x in 0..<width(of: columns, from: column) {
                counts[pixel(x: left(column) + x, y: padding + row * cell.height + y), default: 0] += 1
            }
        }
        return counts
    }

    /// The colour most of the cell's pixels have: its background, whatever a glyph or a line leaves of it.
    func background(row: Int, column: Int) -> RGB {
        histogram(row: row, column: column).max { $0.value < $1.value }?.key ?? RGB(0, 0, 0)
    }

    /// Whether any pixel of the cell is not `background` (a glyph, a line).
    func marks(row: Int, column: Int, over background: RGB, columns: Int = 1) -> Bool {
        histogram(row: row, column: column, columns: columns).contains { $0.key != background }
    }

    /// The pixels of the cell at row `line` (0 at its top) that are not its background, as columns from its left.
    func inked(row: Int, column: Int, line: Int, over background: RGB, columns: Int = 1) -> [Int] {
        (0..<width(of: columns, from: column)).filter {
            pixel(x: left(column) + $0, y: padding + row * cell.height + line) != background
        }
    }
}

extension Instance {
    /// The selected terminal as drawn now: a few forced draws first (a window behind others gets no display-link tick), then
    /// `shot` into the scratch folder. Nil, the failure recorded, when the file or `state` cannot be read.
    func shot(_ name: String) async -> GridShot? {
        let file = scratch.root.appendingPathComponent(name + ".png")
        let said = await send(["wait-idle 2000", "frames 3", "delay 250", "state", "shot \(file.path)"])
        guard let picture = GridShot(png: file, state: said[3]) else {
            fail("no picture of the terminal: \(said)")
            return nil
        }
        return picture
    }
}
