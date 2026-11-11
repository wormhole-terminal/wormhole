// How the app window itself moves while the intro plays: its scale about its centre and its opacity at `t`
// (0 to 1 across the run), for a window that is showing when the intro begins or one that never was.

import Foundation

/// Whether the window is on screen when the intro begins.
public enum IntroStart: Sendable {
    /// A replay: the window is there, and leaves as the intro starts.
    case shown
    /// A first launch: the window was never shown, so it stays out of sight until it arrives.
    case hidden
}

public struct WindowMotion: Equatable, Sendable {
    public var scale: Float
    public var alpha: Float
    public static let atRest = WindowMotion(scale: 1, alpha: 1)

    public init(scale: Float, alpha: Float) { (self.scale, self.alpha) = (scale, alpha) }

    /// A skip settles from where the window was to rest: `settled` is 0 to 1.
    public func settling(_ settled: Float) -> WindowMotion {
        WindowMotion(
            scale: scale + (Self.atRest.scale - scale) * settled, alpha: alpha + (Self.atRest.alpha - alpha) * settled)
    }
}

extension IntroStyle {
    /// Where the FTL jump goes to light speed and drops out, as fractions of its run.
    static let ftlJump: Float = 0.22
    static let ftlExit: Float = 0.76

    public func windowMotion(at t: Float, start: IntroStart = .shown) -> WindowMotion {
        let t = min(max(t, 0), 1)
        switch self {
        // Nothing while the screen sinks into space and the throat opens, then the window comes through it.
        case .emerge:
            let grow = Ease.phase(t, 0.36, 0.74)
            return WindowMotion(scale: 0.08 + 0.92 * Ease.outBack(grow), alpha: Ease.smoothstep(0, 0.45, grow))
        // Appears as the swirl closes in, settling from a touch larger.
        case .collapse:
            let settle = Ease.phase(t, 0.55, 0.85)
            return WindowMotion(scale: 1.05 - 0.05 * Ease.outCubic(settle), alpha: Ease.smoothstep(0, 1, settle))
        // Gone before the jump, and there when it arrives: settling from a little smaller.
        case .ftl:
            let arrive = Ease.phase(t, Self.ftlExit - 0.01, Self.ftlExit + 0.09)
            let leave: Float = start == .shown ? 1 - Ease.smoothstep(0, 0.06, t) : 0
            return WindowMotion(scale: 0.93 + 0.07 * Ease.outCubic(arrive), alpha: max(leave, Ease.smoothstep(0, 1, arrive)))
        case .aurora, .tentacles, .voyage:
            return Wormhole.windowMotion(seconds: t * seconds, start: start)
        case .vortex, .glow:
            return .atRest
        }
    }
}
