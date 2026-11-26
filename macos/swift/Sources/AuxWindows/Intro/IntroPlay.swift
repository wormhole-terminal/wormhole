// text-file: the control folder's replies, not interface text
// What the last `IntroPlayer.play` came to. An intro that does not show leaves nothing to look at, so the player keeps the answer:
// `intro-support` tells it, and a bundle check on a machine of its own reads it to say why no overlay was there.

import Foundation

public enum IntroPlay: Equatable, Sendable {
    /// Nothing has asked for an intro yet.
    case none
    /// Asked, with no window to play it over.
    case noWindow
    /// Asked; the overlay could not be made (`why`, on one line).
    case failed(String)
    /// The overlay was made. `shown` is the window server's answer as it opened: an overlay above a window that is not on screen is not.
    case opened(shown: Bool)

    /// `none`, `no window`, `failed: <why>`, `opened shown` or `opened hidden`, as `intro-support` says it.
    public var line: String {
        switch self {
        case .none: "none"
        case .noWindow: "no window"
        case .failed(let why): "failed: \(why)"
        case .opened(let shown): shown ? "opened shown" : "opened hidden"
        }
    }
}
