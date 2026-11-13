// How a terminal window opens and closes, as the core decides it (`view::motion::window_animation`): the shell maps it
// onto `NSWindow.AnimationBehavior`.

public enum WindowAnimation: Sendable, Equatable {
    /// AppKit's own, as for a window nobody asked anything of.
    case standard
    /// The document window's scale and fade.
    case document
    /// None: a cut.
    case cut
}
