// Where everything is in the minimap (`wormhole_term::minimap::MinimapGeometry`): how many lines there are and
// which of them the view shows. Lines are counted from the oldest line of history (0) to the last line of the screen.
// When they do not all fit, the minimap shows a window of them that moves with the view, as VS Code's does. A port of
// the Rust arithmetic (a dozen lines each) because a drag runs it per mouse event; the FFI's value replaces it.

import CoreGraphics
import Foundation

public struct MinimapGeometry: Equatable, Sendable {
    /// Width of the minimap, in points (`WIDTH`).
    public static let width: CGFloat = 64
    /// Height of one terminal line in the minimap, the block and the gap under it (`LINE_PITCH`).
    public static let linePitch: CGFloat = 2
    /// Height of a block within `linePitch` (`BLOCK_H`).
    public static let blockHeight: CGFloat = 1.5
    /// Widest a column gets, so a narrow terminal's lines are not stretched across the minimap (`COLUMN_MAX`).
    public static let columnMax: CGFloat = 1

    /// History and screen together.
    public var total: Int
    public var screen: Int
    /// Lines the minimap has room for.
    public var fits: Int
    /// The first line on screen.
    public var viewTop: Int

    public init(total: Int, screen: Int, fits: Int, viewTop: Int) {
        (self.total, self.screen, self.fits, self.viewTop) = (total, screen, fits, viewTop)
    }

    private var history: Int { max(total - screen, 0) }

    /// The first line the minimap shows.
    public var first: Int {
        let spare = max(total - fits, 0)
        guard spare > 0, history > 0 else { return 0 }
        return Int((Double(spare) * Double(min(viewTop, history)) / Double(history)).rounded())
    }

    /// The marked part, the lines on screen, as a top and a height in points.
    public var slider: (top: CGFloat, height: CGFloat) {
        (CGFloat(max(viewTop - first, 0)) * Self.linePitch, CGFloat(min(screen, total)) * Self.linePitch)
    }

    /// Whether the view is a window of the lines, so the mark means something.
    public var scrolls: Bool { total > screen }

    /// The line drawn `y` points from the minimap's top, if there is one.
    public func line(atY y: CGFloat) -> Int? {
        let line = first + Int(max(y, 0) / Self.linePitch)
        return line < total ? line : nil
    }

    /// The view's first line once the slider's top is dragged to `y`: the lines' own place while they all fit, else
    /// a scrollbar's, over the room the slider has to move in.
    public func viewTop(forSliderY y: CGFloat) -> Int {
        let track = CGFloat(max(fits - screen, 0)) * Self.linePitch
        let top = (total <= fits || track <= 0) ? (y / Self.linePitch).rounded() : (y / track * CGFloat(history)).rounded()
        return min(max(Int(top), 0), history)
    }

    /// The view's first line that puts `line` in the middle of the screen.
    public func viewTop(centering line: Int) -> Int { min(max(line - screen / 2, 0), history) }

    /// How far into the history the view is scrolled with `viewTop` as its first line.
    public func displayOffset(viewTop: Int) -> Int { history - min(viewTop, history) }
}
