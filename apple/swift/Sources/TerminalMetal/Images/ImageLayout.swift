// Where a placement draws, in device pixels, and which part of its texture shows: the geometry of the GPUI
// app's `render/images.rs` (`placement_paint`), done once here so the shader only samples. Pure, so it is
// tested without a GPU.

import TerminalCore

/// One picture's quad: the rectangle on the surface and the part of the texture that fills it (0...1).
struct ImageQuad: Equatable {
    var x: Float
    var y: Float
    var width: Float
    var height: Float
    var u0: Float
    var v0: Float
    var u1: Float
    var v1: Float
}

/// What the layout needs to know about the surface, all in device pixels except `scale`.
struct ImageSurface {
    var cellWidth: Double
    var cellHeight: Int
    /// Device pixels per point: the engine measures offsets and pixel extents in the points it was told a cell has.
    var scale: Double
    /// The grid's own area: a picture that scrolled partly out of view must not draw past it.
    var width: Int
    var height: Int
}

enum ImageLayout {
    /// The quad for `placement` showing a picture of `imageWidth` x `imageHeight` pixels, with the view scrolled
    /// `displayOffset` rows into the history; `nil` when nothing of it is in view.
    static func quad(
        _ placement: ImagePlacement, imageWidth: Int, imageHeight: Int, displayOffset: Int, surface: ImageSurface
    ) -> ImageQuad? {
        guard imageWidth > 0, imageHeight > 0 else { return nil }
        let cw = surface.cellWidth, ch = Double(surface.cellHeight)
        let row = Double(placement.line + displayOffset)
        let corner = (x: cw * Double(placement.column), y: ch * row)
        let cells = (width: cw * Double(placement.cols), height: ch * Double(placement.rows))
        let full = (width: Double(imageWidth), height: Double(imageHeight))
        let source = placement.source.width > 0 && placement.source.height > 0
            ? placement.source : ImageRect(width: imageWidth, height: imageHeight)
        let offset = (x: Double(placement.offsetX) * surface.scale, y: Double(placement.offsetY) * surface.scale)
        let start = (x: corner.x + offset.x, y: corner.y + offset.y)

        let shown: (x: Double, y: Double, width: Double, height: Double)
        switch placement.extent {
        case .fill:
            shown = (start.x, start.y, max(cells.width - offset.x, 1), max(cells.height - offset.y, 1))
        case .fit:
            // The part's shape at the largest size that fits the cells, centred in them.
            let scale = min(cells.width / Double(source.width), cells.height / Double(source.height))
            let drawn = (width: Double(source.width) * scale, height: Double(source.height) * scale)
            shown = (
                corner.x + (cells.width - drawn.width) / 2, corner.y + (cells.height - drawn.height) / 2, drawn.width, drawn.height
            )
        case .pixels(let width, let height):
            shown = (start.x, start.y, Double(width) * surface.scale, Double(height) * surface.scale)
        }
        guard shown.width > 0, shown.height > 0 else { return nil }

        // Cut to the grid; the texture's coordinates follow the cut so the picture stays where it was laid.
        let left = max(shown.x, 0), top = max(shown.y, 0)
        let right = min(shown.x + shown.width, Double(surface.width)), bottom = min(shown.y + shown.height, Double(surface.height))
        guard right > left, bottom > top else { return nil }
        func u(_ x: Double) -> Float { Float((Double(source.x) + (x - shown.x) / shown.width * Double(source.width)) / full.width) }
        func v(_ y: Double) -> Float { Float((Double(source.y) + (y - shown.y) / shown.height * Double(source.height)) / full.height) }
        return ImageQuad(
            x: Float(left), y: Float(top), width: Float(right - left), height: Float(bottom - top), u0: u(left), v0: v(top),
            u1: u(right), v1: v(bottom))
    }
}
