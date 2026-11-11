// Exact, cell-sized shapes for box drawing (U+2500...257F) and block elements (U+2580...259F): drawn by the renderer so
// lines meet across cells whatever the font does at the edges. The rounded corners (U+256D...2570) and braille
// (U+2800...28FF) are the font's, as in the GPUI app: its arcs and its small dots, which a square corner or a larger
// dot would not match. The dashes are plain.

import CoreGraphics
import Foundation

enum BoxDrawing {
    static func handles(_ scalar: UInt32) -> Bool {
        (0x2500...0x259F).contains(scalar) && !(0x256D...0x2570).contains(scalar)
    }

    /// The glyph as a cell-sized bitmap whose origin is the cell's top-left corner.
    static func raster(_ scalar: UInt32, metrics: CellMetrics) -> Raster {
        let w = metrics.width, h = metrics.height
        let pixels = GlyphRaster.coverage(width: w, height: h) { context in
            // Work top-down, like the cell.
            context.translateBy(x: 0, y: CGFloat(h))
            context.scaleBy(x: 1, y: -1)
            context.setFillColor(gray: 1, alpha: 1)
            context.setShouldAntialias(false)
            let canvas = Canvas(context: context, width: w, height: h)
            switch scalar {
            case 0x2500...0x257F: lines(scalar, canvas, light: max(1, Int((Double(w) / 8).rounded())))
            default: blocks(scalar, canvas)
            }
        }
        // The cell's top is `baseline` above the baseline.
        return Raster(width: w, height: h, left: 0, top: metrics.baseline, isColor: false, pixels: pixels)
    }

    struct Canvas {
        let context: CGContext
        let width: Int
        let height: Int

        func fill(_ x0: Int, _ y0: Int, _ x1: Int, _ y1: Int, alpha: CGFloat = 1) {
            guard x1 > x0, y1 > y0 else { return }
            context.setAlpha(alpha)
            context.fill(CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0))
            context.setAlpha(1)
        }
    }

    // MARK: blocks

    private static func blocks(_ scalar: UInt32, _ c: Canvas) {
        let w = c.width, h = c.height
        func eighths(_ n: Int, of total: Int) -> Int { Int((Double(total) * Double(n) / 8).rounded()) }
        switch scalar {
        case 0x2580: c.fill(0, 0, w, h / 2)
        case 0x2581...0x2588: c.fill(0, h - eighths(Int(scalar) - 0x2580, of: h), w, h)
        case 0x2589...0x258F: c.fill(0, 0, eighths(0x2590 - Int(scalar), of: w), h)
        case 0x2590: c.fill(w / 2, 0, w, h)
        case 0x2591: c.fill(0, 0, w, h, alpha: 0.25)
        case 0x2592: c.fill(0, 0, w, h, alpha: 0.5)
        case 0x2593: c.fill(0, 0, w, h, alpha: 0.75)
        case 0x2594: c.fill(0, 0, w, eighths(1, of: h))
        case 0x2595: c.fill(w - eighths(1, of: w), 0, w, h)
        default:
            // Quadrants: 1 upper left, 2 upper right, 4 lower left, 8 lower right.
            let masks: [UInt32: Int] = [
                0x2596: 4, 0x2597: 8, 0x2598: 1, 0x2599: 13, 0x259A: 9, 0x259B: 7, 0x259C: 11, 0x259D: 2,
                0x259E: 6, 0x259F: 14,
            ]
            let mask = masks[scalar] ?? 0
            if mask & 1 != 0 { c.fill(0, 0, w / 2, h / 2) }
            if mask & 2 != 0 { c.fill(w / 2, 0, w, h / 2) }
            if mask & 4 != 0 { c.fill(0, h / 2, w / 2, h) }
            if mask & 8 != 0 { c.fill(w / 2, h / 2, w, h) }
        }
    }

    // MARK: lines

    /// Left, right, up, down weights per box-drawing character from U+2500: 0 none, 1 light, 2 heavy, 3 double.
    private static let arms: [String] = [
        "1100", "2200", "0011", "0022", "1100", "2200", "0011", "0022", "1100", "2200", "0011", "0022",
        "0101", "0201", "0102", "0202", "1001", "2001", "1002", "2002", "0110", "0210", "0120", "0220",
        "1010", "2010", "1020", "2020", "0111", "0211", "0121", "0112", "0122", "0221", "0212", "0222",
        "1011", "2011", "1021", "1012", "1022", "2021", "2012", "2022", "1101", "2101", "1201", "2201",
        "1102", "2102", "1202", "2202", "1110", "2110", "1210", "2210", "1120", "2120", "1220", "2220",
        "1111", "2111", "1211", "2211", "1121", "1112", "1122", "2121", "1221", "2112", "1212", "2221",
        "2212", "2122", "1222", "2222", "1100", "2200", "0011", "0022", "3300", "0033", "0301", "0103",
        "0303", "3001", "1003", "3003", "0310", "0130", "0330", "3010", "1030", "3030", "0311", "0133",
        "0333", "3011", "1033", "3033", "3301", "1103", "3303", "3310", "1130", "3330", "3311", "1133",
        "3333", "0101", "1001", "1010", "0110", "0000", "0000", "0000", "1000", "0010", "0100", "0001",
        "2000", "0020", "0200", "0002", "1200", "0012", "2100", "0021",
    ]

    private static func lines(_ scalar: UInt32, _ c: Canvas, light: Int) {
        let index = Int(scalar - 0x2500)
        if scalar == 0x2571 || scalar == 0x2572 || scalar == 0x2573 { return diagonals(scalar, c, light: light) }
        let digits = arms[index].utf8.map { Int($0 - 48) }
        let (left, right, up, down) = (digits[0], digits[1], digits[2], digits[3])
        let dashes: Int? =
            switch index {
            case 0x04...0x07: 3
            case 0x08...0x0B: 4
            case 0x4C...0x4F: 2
            default: nil
            }
        if let dashes {
            dashed(horizontal: left > 0, weight: max(left, up), dashes: dashes, c, light: light)
            return
        }
        // Horizontal arms: along x, across y.
        axis(
            neg: left, pos: right, crossNeg: up, crossPos: down, along: c.width, across: c.height, light: light
        ) { a0, a1, b0, b1 in c.fill(a0, b0, a1, b1) }
        // Vertical arms: the same with x and y swapped.
        axis(
            neg: up, pos: down, crossNeg: left, crossPos: right, along: c.height, across: c.width, light: light
        ) { a0, a1, b0, b1 in c.fill(b0, a0, b1, a1) }
    }

    private static func span(_ weight: Int, light: Int) -> Int { weight == 0 ? 0 : weight == 2 ? light * 2 : weight == 3 ? light * 3 : light }

    /// Both arms of one axis. `fill(alongStart, alongEnd, acrossStart, acrossEnd)` draws a rectangle in this
    /// axis's own coordinates. The cases are what makes a double line meet its neighbours: an outer line
    /// runs to the outer edge of the junction, an inner one stops at the inner edge (the standard shapes of
    /// U+2550...256C).
    private static func axis(
        neg: Int, pos: Int, crossNeg: Int, crossPos: Int, along: Int, across: Int, light: Int,
        fill: (Int, Int, Int, Int) -> Void
    ) {
        let centre = along / 2, crossCentre = across / 2
        let crossWeight = max(crossNeg, crossPos)
        let junction = span(crossWeight, light: light)
        let lo = centre - junction / 2, hi = lo + junction
        let onlyPos = crossPos > 0 && crossNeg == 0, onlyNeg = crossNeg > 0 && crossPos == 0

        /// The bands an arm of `weight` draws across the axis: one, or two for a double line.
        func bands(_ weight: Int) -> [(Int, Int)] {
            let width = span(weight, light: light)
            let start = crossCentre - width / 2
            return weight == 3 ? [(start, start + light), (start + 2 * light, start + 3 * light)] : [(start, start + width)]
        }

        for (weight, isPositive) in [(neg, false), (pos, true)] where weight > 0 {
            for (band, (b0, b1)) in bands(weight).enumerated() {
                var edge: Int
                if crossWeight == 0 {
                    edge = centre
                } else if crossWeight == 3 && weight == 3 {
                    // Corner of two double lines: the outer band takes the junction's outer edge.
                    let outer = (onlyPos && band == 0) || (onlyNeg && band == 1)
                    if isPositive {
                        edge = outer ? lo : hi - light
                    } else {
                        edge = outer ? hi : lo + light
                    }
                } else if crossWeight == 3 {
                    edge = isPositive ? hi : lo
                } else {
                    edge = isPositive ? lo : hi
                }
                // A straight double line has no junction at all: it runs through.
                if crossWeight == 0 { edge = centre }
                if isPositive { fill(edge, along, b0, b1) } else { fill(0, edge, b0, b1) }
            }
        }
    }

    private static func dashed(horizontal: Bool, weight: Int, dashes: Int, _ c: Canvas, light: Int) {
        let thickness = span(weight == 2 ? 2 : 1, light: light)
        let along = horizontal ? c.width : c.height
        let across = horizontal ? c.height : c.width
        let step = Double(along) / Double(dashes)
        let dash = max(1, Int((step * 0.55).rounded()))
        let start = across / 2 - thickness / 2
        for index in 0..<dashes {
            let a0 = Int((Double(index) * step + (step - Double(dash)) / 2).rounded())
            if horizontal { c.fill(a0, start, a0 + dash, start + thickness) } else { c.fill(start, a0, start + thickness, a0 + dash) }
        }
    }

    private static func diagonals(_ scalar: UInt32, _ c: Canvas, light: Int) {
        c.context.setShouldAntialias(true)
        c.context.setStrokeColor(gray: 1, alpha: 1)
        c.context.setLineWidth(CGFloat(light))
        let w = CGFloat(c.width), h = CGFloat(c.height)
        if scalar != 0x2572 {
            c.context.move(to: CGPoint(x: 0, y: h))
            c.context.addLine(to: CGPoint(x: w, y: 0))
        }
        if scalar != 0x2571 {
            c.context.move(to: CGPoint(x: 0, y: 0))
            c.context.addLine(to: CGPoint(x: w, y: h))
        }
        c.context.strokePath()
    }
}
