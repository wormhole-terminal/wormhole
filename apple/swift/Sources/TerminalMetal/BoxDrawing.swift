// Box drawing (U+2500...257F) and block elements (U+2580...259F) as the GPUI app paints them: the core's shapes for the cell
// (`CellShapes`), rasterised once per cell width and phase (a 16.8 px cell starts on a different fraction of a pixel in each of
// five columns, and GPUI snaps a half block's edge on the fractional cell). Without the core the font draws them.

import CoreGraphics
import TerminalCore

enum BoxDrawing {
    static func handles(_ scalar: UInt32) -> Bool { (0x2500...0x259F).contains(scalar) && CellShapes.current != nil }

    /// The character as a bitmap of its cell (`width` whole pixels from its left whole pixel, `phase` the fractional cell's
    /// offset past it), origin the cell's top-left; nil when the core leaves it to the font.
    static func raster(
        _ scalar: UInt32, width: Int, metrics: CellMetrics, cellWidth: Double, phase: Double, fontSize: Double, scale: Double
    ) -> Raster? {
        let h = metrics.height
        guard let source = CellShapes.current, width > 0,
            let (shapes, alpha) = source.shapes(
                scalar, cellWidth: Float(cellWidth), height: Float(h), phase: Float(phase), fontSize: Float(fontSize), scale: Float(scale))
        else { return nil }
        let pixels = GlyphRaster.coverage(width: width, height: h) { context in
            // Work top-down, like the cell.
            context.translateBy(x: 0, y: CGFloat(h))
            context.scaleBy(x: 1, y: -1)
            context.setFillColor(gray: 1, alpha: CGFloat(alpha))
            context.setStrokeColor(gray: 1, alpha: CGFloat(alpha))
            context.setLineCap(.butt)
            for shape in shapes {
                if shape.rect.count == 4 {
                    let r = shape.rect.map { CGFloat($0) }
                    context.fill(CGRect(x: r[0], y: r[1], width: r[2] - r[0], height: r[3] - r[1]))
                } else if shape.points.count >= 4 {
                    context.setLineWidth(CGFloat(shape.width))
                    context.move(to: CGPoint(x: CGFloat(shape.points[0]), y: CGFloat(shape.points[1])))
                    for i in stride(from: 2, to: shape.points.count - 1, by: 2) {
                        context.addLine(to: CGPoint(x: CGFloat(shape.points[i]), y: CGFloat(shape.points[i + 1])))
                    }
                    context.strokePath()
                }
            }
        }
        // The cell's top is `baseline` above the baseline.
        return Raster(width: width, height: h, left: 0, top: metrics.baseline, isColor: false, pixels: pixels)
    }
}
