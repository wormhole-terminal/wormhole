// Where a window is, as the core keeps it and as the shell reads it (core `view::layout::frame`): plain numbers in one
// global space with y growing downwards and the primary display first, so a record means the same on any machine and
// in either shell. The shell converts to and from AppKit's frames at the edge (`WindowUI.WindowFrames`).

import Foundation

public enum FrameKind: Sendable, Equatable {
    case windowed, maximized, fullscreen
}

public struct WindowFrame: Sendable, Equatable {
    public var kind: FrameKind
    public var x: Double
    public var y: Double
    public var width: Double
    public var height: Double
    /// The display it was on; the rectangle is global, this only disambiguates.
    public var display: UInt64?
    /// It was in the Dock; the rectangle is where it comes back to.
    public var minimised: Bool

    public init(
        kind: FrameKind = .windowed, x: Double, y: Double, width: Double, height: Double, display: UInt64? = nil,
        minimised: Bool = false
    ) {
        (self.kind, self.x, self.y, self.width, self.height) = (kind, x, y, width, height)
        (self.display, self.minimised) = (display, minimised)
    }
}

/// A display's usable area: the menu bar and the Dock taken off.
public struct DisplayArea: Sendable, Equatable {
    public var id: UInt64
    public var x: Double
    public var y: Double
    public var width: Double
    public var height: Double

    public init(id: UInt64, x: Double, y: Double, width: Double, height: Double) {
        (self.id, self.x, self.y, self.width, self.height) = (id, x, y, width, height)
    }
}
