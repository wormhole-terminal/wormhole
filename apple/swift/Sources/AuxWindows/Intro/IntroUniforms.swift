// `Uniforms` in `shader/common.metal`, field for field: float2 fields are 8-aligned and float4s 16-aligned, which
// Swift's SIMD types give too (a test checks the size). One frame's numbers are the style's timing at `t`.

import AuxModel
import DesignSystem
import Foundation

struct IntroUniforms {
    var size: SIMD2<Float> = .zero
    var center: SIMD2<Float> = .zero
    var halfSize: SIMD2<Float> = .zero
    var scale: Float = 1
    var t: Float = 0
    var time: Float = 0
    var winScale: Float = 1
    var radius: Float = 12
    var winAlpha: Float = 1
    var style: UInt32 = 0
    var speed: Float = 0
    var travel: Float = 0
    /// Voyage's mist: 1 for the smooth tunnel and blurred arms, 0 for the others.
    var mist: Float = 0
    /// The wormhole styles' run: its seconds, when the light lifts, the bloom's delay, the arms' size.
    var run: SIMD4<Float> = .zero
    /// Voyage's ship: x, y, its icon box's side, heading; then thrust, opacity, flare, bank.
    var ship: SIMD4<Float> = .zero
    var shipFX: SIMD4<Float> = .zero
    /// The theme's primary and info colours and its terminal magenta (FTL's violet).
    var c1: SIMD4<Float> = .zero
    var c2: SIMD4<Float> = .zero
    var c3: SIMD4<Float> = .zero

    /// macOS's window corners, near enough: the glow hugs them.
    static let cornerRadius: Float = 12

    /// For a screen of `screen` points with a window at `origin` (from the screen's top left) of `size` points.
    init(screen: SIMD2<Float>, origin: SIMD2<Float>, size: SIMD2<Float>, scale: Float, colors: IntroColors) {
        self.size = screen
        center = origin + size / 2
        halfSize = size / 2
        self.scale = scale
        radius = Self.cornerRadius
        (c1, c2, c3) = (colors.primary, colors.info, colors.magenta)
    }

    /// The style's frame at `t` (0 to 1 across the run), `time` seconds in, for a window `start`ing shown or hidden.
    func at(_ style: IntroStyle, t: Float, time: Float, start: IntroStart) -> IntroUniforms {
        var u = self
        u.style = style.shaderID
        (u.t, u.time) = (t, time)
        let motion = style.windowMotion(at: t, start: start)
        (u.winScale, u.winAlpha) = (motion.scale, motion.alpha)
        u.speed = style.speed(at: t)
        u.travel = style.travel(at: t)
        let run = style.run
        u.run = [run.seconds, run.lift, run.bloom, run.arms]
        u.mist = run.mist
        return u
    }

    /// Voyage's ship at `time` and the comet trail behind it. Any other style has no ship, and neither has the
    /// Portal hand-over (`ship` false): its trail is moved off the screen, as the shader draws the trail whatever
    /// the ship's opacity.
    func withShip(
        _ style: IntroStyle, ship hasShip: Bool, time: Float, screen: SIMD2<Float>, landing: Landing
    ) -> (IntroUniforms, [SIMD2<Float>]) {
        var u = self
        guard style == .voyage else { return (u, Array(repeating: .zero, count: Voyage.trailPoints)) }
        guard hasShip else { return (u, Array(repeating: SIMD2(repeating: -1e5), count: Voyage.trailPoints)) }
        let ship = Voyage.ship(at: time, screen: screen, landing: landing)
        u.ship = [ship.pos.x, ship.pos.y, ship.size, ship.heading]
        u.shipFX = [ship.thrust, ship.opacity, ship.flare, ship.bank]
        return (u, Voyage.trail(at: time, screen: screen, landing: landing))
    }
}

/// The theme's primary and info colours and its terminal magenta, as the shader's RGBA.
public struct IntroColors: Equatable {
    var primary: SIMD4<Float>
    var info: SIMD4<Float>
    var magenta: SIMD4<Float>
}

extension IntroColors {
    /// From the theme's tokens: `primary`, `status.info` and the terminal's ANSI magenta (FTL's violet).
    public init(tokens: ThemeTokens) {
        func simd(_ rgba: RGBA?) -> SIMD4<Float> {
            guard let rgba else { return SIMD4(1, 0, 1, 1) }
            return [Float(rgba.red), Float(rgba.green), Float(rgba.blue), Float(rgba.alpha)]
        }
        self.init(
            primary: simd(tokens.rgba(named: "primary")), info: simd(tokens.rgba(named: "status.info")),
            magenta: simd(tokens.terminal.ansi[5]))
    }
}
