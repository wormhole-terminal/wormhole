// The wormhole run that Aurora, Tentacles and Voyage share, in seconds: the funnel, the jump, the portal and the
// window's arrival. The shader reads these same moments (`ShaderConstants`), so the window, the funnel's flow
// and the light never drift apart.
//
//   0 - gather - jump ------ cruise ------ exit - iris - bloom - hold - lift - end

import Foundation

public enum Wormhole {
    /// The whole run.
    public static let seconds: Float = 8.8
    /// By here the funnel is gathered round its throat.
    public static let gather: Float = 1.2
    /// The flash, and the push to light speed that follows it.
    public static let jump: Float = 1.25
    /// The drop out: speed ends dead, space springs back and the portal opens.
    public static let exit: Float = 4.6
    /// When the window arrives, just behind the flash of the drop out.
    public static let arrive: Float = exit + 0.12
    /// When the light behind the window begins to rise away and fade.
    public static let lift: Float = 7.5

    /// Gone while the wormhole plays (leaving at the start when it was showing), then arriving with a spring.
    static func windowMotion(seconds: Float, start: IntroStart) -> WindowMotion {
        let leave: Float = start == .shown ? 1 - Ease.smoothstep(0, 0.3, seconds) : 0
        if seconds < arrive { return WindowMotion(scale: 1, alpha: leave) }
        let since = seconds - arrive
        let spring = 1 - 0.14 * exp(-5 * since) * cos(11 * since)
        return WindowMotion(scale: spring, alpha: max(leave, Ease.smoothstep(0, 0.25, since)))
    }

    /// How fast the funnel's filaments flow at `seconds`, in log-depth units a second: a drift inward that
    /// settles, a hard push into light speed at the jump, and at the drop out a stop with a pull the other way.
    static func speed(seconds s: Float) -> Float {
        let settling = 0.9 + 1.6 * exp(-s / 0.45)
        let drift = -settling * (1 - Ease.smoothstep(jump - 0.45, jump + 0.15, s))
        let go = Ease.smoothstep(jump - 0.2, jump + 0.55, s)
        let rush = 13 * go * (1 - Ease.smoothstep(exit - 0.05, exit + 0.08, s))
        let back = Ease.phase(s, exit, exit + 0.45)
        let spring: Float = back > 0 && back < 1 ? -4 * sin(Float.pi * back) : 0
        return drift + rush + spring
    }
}
