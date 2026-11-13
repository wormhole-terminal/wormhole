// What one window is drawn with: the look of the profile it is in, and through it that profile's theme. A holder, because
// a window can move to another profile while it is open ("Move Window to"): the look is swapped here and the window's
// root reads it again, with nothing of the window rebuilt.

import Observation

@MainActor @Observable
public final class WindowDress {
    public var look: WindowLook

    public init(_ look: WindowLook = WindowLook()) { self.look = look }
}
