// Where a picture sits in the grid, as the packed frame's placements section says it (`wormhole-ffi`'s
// `inline_images/section.rs`). The engine owns the placement; this is what it tells the view.

import Foundation

/// A part of a picture, in its pixels.
public struct ImageRect: Sendable, Equatable {
    public var x: Int
    public var y: Int
    public var width: Int
    public var height: Int

    public init(x: Int = 0, y: Int = 0, width: Int, height: Int) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
    }
}

/// How large the shown part of the picture is drawn inside the cells the placement covers.
public enum ImageExtent: Sendable, Equatable {
    /// Stretched over the covered cells from the pixel offset on (a program that gave both columns and rows).
    case fill
    /// Scaled to fit inside the covered cells keeping its shape, centred (iTerm2's `preserveAspectRatio`).
    case fit
    /// At this size from the pixel offset on. Points, as the engine measures a cell in the points it was told.
    case pixels(width: Int, height: Int)
}

public struct ImagePlacement: Sendable, Equatable {
    /// The picture's key for `FrameSource.inlineImage`.
    public var key: UInt32
    /// Stacking order: negative is drawn under the text, the lower of two first.
    public var z: Int32
    /// The top row as a grid line: 0 is the live screen's top row, negative is history. Add the frame's display
    /// offset for the viewport row.
    public var line: Int
    public var column: Int
    /// Cells covered, already cut to the screen's width.
    public var cols: Int
    public var rows: Int
    /// The part of the picture shown; zero-sized means all of it.
    public var source: ImageRect
    /// Points from the first cell's corner to the picture's corner.
    public var offsetX: Int
    public var offsetY: Int
    public var extent: ImageExtent

    public init(
        key: UInt32, z: Int32 = 0, line: Int, column: Int, cols: Int, rows: Int,
        source: ImageRect = ImageRect(width: 0, height: 0), offsetX: Int = 0, offsetY: Int = 0,
        extent: ImageExtent = .fill
    ) {
        self.key = key
        self.z = z
        self.line = line
        self.column = column
        self.cols = cols
        self.rows = rows
        self.source = source
        self.offsetX = offsetX
        self.offsetY = offsetY
        self.extent = extent
    }
}

/// A decoded picture: straight (not premultiplied) RGBA, row-major, `width * height * 4` bytes.
public struct InlineImage: Sendable, Equatable {
    public var width: Int
    public var height: Int
    public var rgba: Data

    public init(width: Int, height: Int, rgba: Data) {
        self.width = width
        self.height = height
        self.rgba = rgba
    }
}
