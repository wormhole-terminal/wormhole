// The easing shared by every part of the intro's timing (`wormhole-core` `onboarding::intro`, the private
// `phase`, `smoothstep`, `ease_out_*` helpers). Plain functions on Float: the shader's uniforms are floats.

import Foundation

enum Ease {
    /// `t` mapped so `from` is 0 and `to` is 1, clamped.
    static func phase(_ t: Float, _ from: Float, _ to: Float) -> Float {
        min(max((t - from) / (to - from), 0), 1)
    }

    static func smoothstep(_ from: Float, _ to: Float, _ t: Float) -> Float {
        let x = phase(t, from, to)
        return x * x * (3 - 2 * x)
    }

    static func outCubic(_ x: Float) -> Float { 1 - pow(1 - x, 3) }

    /// Overshoots a little before settling: the window lands rather than stops.
    static func outBack(_ x: Float) -> Float {
        let c1: Float = 1.2
        let c3 = c1 + 1
        return 1 + c3 * pow(x - 1, 3) + c1 * pow(x - 1, 2)
    }
}
