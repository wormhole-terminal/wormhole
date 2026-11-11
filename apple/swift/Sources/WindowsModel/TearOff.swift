// What letting go of a terminal outside its window means, as plain values: which window of the app lies under the pointer
// and where the window of a terminal torn off opens. The rules are the core's (`view::tear_off`); the shell reads the
// windows and screens from AppKit, asks the backend and does what it answers. Every point is in AppKit's global space
// (y up) except `tearOffPlace`'s, which is in the space of one display (from its top left, y down).

import CoreGraphics

/// A window of the app on the screens (the stack the backend is given runs front to back).
public struct TearOffWindow: Sendable, Equatable {
    /// Set when the window shows terminals that can take one (the dragging window's profile, in sight); nil for any other
    /// window of the app (Settings, the Theme Studio), which hides what lies behind it and takes nothing.
    public var window: WindowID?
    public var bounds: CGRect

    public init(window: WindowID?, bounds: CGRect) { (self.window, self.bounds) = (window, bounds) }
}

public enum TearOffLanding: Sendable, Equatable {
    /// Nothing: over its own window, or one that takes nothing.
    case stay
    /// Over no window of the app: the terminal opens a window of its own there.
    case newWindow
    /// Over another window that shows terminals: the terminal goes into it.
    case into(WindowID)
}

/// A torn-off window as it opens: its place in the display's space and the chip it was let go as, in the window's own.
public struct TearOffPlaced: Sendable, Equatable {
    public var bounds: CGRect
    public var chip: CGRect

    public init(bounds: CGRect, chip: CGRect) { (self.bounds, self.chip) = (bounds, chip) }
}

/// What shows of a window growing from its chip (`view::motion::tear_off`), in the window's own space (y down).
public struct TearOffPose: Sendable, Equatable {
    public var shown: CGRect
    public var radius: CGFloat
    /// How much of the window's content shows, and how much of the chip's face (tile and name): 0 to 1.
    public var content: Double
    public var face: Double
    public var done: Bool

    public init(shown: CGRect, radius: CGFloat, content: Double, face: Double, done: Bool) {
        (self.shown, self.radius, self.content, self.face, self.done) = (shown, radius, content, face, done)
    }
}
