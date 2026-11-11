// How fast the style's flow goes and how far it has gone: space past the window in the FTL jump, the funnel's
// filaments in the wormhole styles. The shader scrolls by `travel`, so it is the integral of `speed`.

import Foundation

extension IntroStyle {
    /// How fast the flow goes at `t` (0 to 1 across the run); 0 for the styles without one.
    public func speed(at t: Float) -> Float {
        switch self {
        case .ftl: Self.ftlSpeed(t)
        case .aurora, .tentacles, .voyage: Wormhole.speed(seconds: t * seconds)
        default: 0
        }
    }

    /// How far the flow has gone by `t`: the integral of `speed` over the run's seconds.
    public func travel(at t: Float) -> Float {
        let steps = 240
        let t = min(max(t, 0), 1)
        let dt = t / Float(steps)
        var sum: Float = 0
        for i in 0..<steps { sum += speed(at: (Float(i) + 0.5) * dt) }
        return sum * dt * seconds
    }

    /// Points a second: a slow drift, a hard push to light speed at the jump, a cruise, the sudden drop out.
    static func ftlSpeed(_ t: Float) -> Float {
        let drift: Float = 14
        let cruise: Float = 5200
        let up = Ease.smoothstep(ftlJump - 0.04, ftlJump + 0.02, t)
        let down = Ease.smoothstep(ftlExit - 0.015, ftlExit + 0.03, t)
        return drift + (cruise - drift) * up * (1 - down)
    }
}
