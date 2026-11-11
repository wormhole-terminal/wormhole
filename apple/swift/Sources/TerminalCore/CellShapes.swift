// Box drawing and block elements as the GPUI app paints them: exact shapes made for the cell (`wormhole_term::glyphs`, its
// painter's snapping), so lines and blocks meet across cells whatever the font does. `UniFFISource` installs the core's; a
// build without Rust has none and those characters come from the font.

import Synchronization

/// A filled rectangle (`rect`: left, top, right, bottom) or a stroked polyline with butt caps (`points`, `width`), in device
/// pixels from the cell's left whole pixel and its top.
public struct CellShape: Equatable, Sendable {
    public var rect: [Float]
    public var points: [Float]
    public var width: Float

    public init(rect: [Float] = [], points: [Float] = [], width: Float = 0) { (self.rect, self.points, self.width) = (rect, points, width) }
}

public protocol CellShapeSource: Sendable {
    /// The shapes of `scalar` in a cell `cellWidth` x `height` pixels starting `phase` past a whole pixel, and their opacity;
    /// nil for a character the font draws.
    func shapes(_ scalar: UInt32, cellWidth: Float, height: Float, phase: Float, fontSize: Float, scale: Float) -> (shapes: [CellShape], alpha: Float)?
}

public enum CellShapes {
    private static let installed = Mutex<(any CellShapeSource)?>(nil)

    public static func install(_ source: any CellShapeSource) { installed.withLock { $0 = source } }
    public static var current: (any CellShapeSource)? { installed.withLock { $0 } }
}
