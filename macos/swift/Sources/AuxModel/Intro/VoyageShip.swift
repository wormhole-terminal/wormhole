// The ship at one moment of Voyage, and the comet trail behind it (`voyage::ship`, `voyage::trail`). The shader
// draws what `Ship` says; the path is a cruise (rise, hover, dive, sway) and then a Bezier swoop that comes in on
// the logo's chevron from its left, shrinking as it recedes.

import Foundation

public struct Ship: Equatable, Sendable {
    /// Where its chevron's middle is, points from the screen's top left.
    public var pos: SIMD2<Float>
    /// The side of the icon box it would be drawn in: at the dock, the logo's.
    public var size: Float
    /// Radians clockwise from pointing up; a quarter turn is the icon's `>`.
    public var heading: Float
    public var bank: Float
    /// How hard the engines burn, an afterburner flash at the jump and drop out, and how solid it is (0 to 1).
    public var thrust: Float
    public var opacity: Float
    public var flare: Float
}

extension Voyage {
    /// How wide the ship's chevron is in front of the camera, as a fraction of the screen's height.
    private static let wide: Float = 0.3924
    private static let big: Float = wide * iconCanvas / (2 * armX + iconStroke)
    /// Where the ship hovers and cruises, as fractions of the screen's height below the funnel's centre.
    private static let hover: Float = 0.30
    private static let cruiseAt: Float = 0.26
    private static let cruiseScale: Float = 0.72
    private static let launch: Float = 1.1

    /// The ship's place, size and heading before it banks: from the launch to the swoop's end.
    private static func raw(_ s: Float, _ screen: SIMD2<Float>, _ landing: Landing) -> (SIMD2<Float>, Float, Float) {
        let (w, h) = (screen.x, screen.y)
        let centre = screen / 2
        func cruise(_ s: Float) -> (SIMD2<Float>, Float, Float) {
            let rise = Ease.outCubic(Ease.phase(s, 0, launch))
            let dive = Ease.smoothstep(Wormhole.jump - 0.05, Wormhole.jump + 0.8, s)
            let settled = Ease.smoothstep(Wormhole.jump, Wormhole.jump + 1, s)
            let hoverY = centre.y + hover * h
            var y = h * 1.28 + (hoverY - h * 1.28) * rise
            y += (centre.y + cruiseAt * h - hoverY) * dive
            y += 0.012 * h * sin(1.7 * s) * settled
            let x = centre.x + 0.035 * w * sin(1.1 * s + 0.4) * settled
            var kick: Float = 1
            if s > Wormhole.exit {
                let d = s - Wormhole.exit
                kick = 1 + 0.10 * exp(-5 * d) * sin(12 * d)
            }
            let size = big * h * (1 - (1 - cruiseScale) * dive) * (1 + 0.025 * sin(2.3 * s) * settled) * kick
            return ([x, y], size, 0.07 * cos(1.1 * s + 0.4) * settled)
        }
        if s < swoop { return cruise(s) }
        let (p0, size0, heading0) = cruise(swoop)
        let target = landing.chevron
        let p1: SIMD2<Float> = [p0.x - 0.10 * w, p0.y - 0.50 * (p0.y - target.y)]
        let p2: SIMD2<Float> = [target.x - 0.24 * w, target.y + 0.08 * h]
        let tau = Ease.smoothstep(0, 1, Ease.phase(s, swoop, dock))
        let u = 1 - tau
        let pos = p0 * (u * u * u) + p1 * (3 * u * u * tau) + p2 * (3 * u * tau * tau) + target * (tau * tau * tau)
        // The nose comes round to point right, leaning into the veer: a ship yawing steadily.
        let heading =
            heading0 + (Float.pi / 2 - heading0) * Ease.smoothstep(0.05, 1, tau) - 0.5 * sin(Float.pi * tau)
        let shrink = pow(Ease.smoothstep(0, 1, tau), 0.8)
        let size = exp(log(size0) + (log(landing.canvas) - log(size0)) * shrink)
        let touchdown = 1 + 0.12 * exp(-pow((s - land) / 0.08, 2))
        return s >= dock ? (target, landing.canvas * touchdown, Float.pi / 2) : (pos, size, heading)
    }

    /// The ship at `seconds` into the run, on a screen of `screen` points with the logo where `landing` says.
    public static func ship(at s: Float, screen: SIMD2<Float>, landing: Landing) -> Ship {
        let (pos, size, heading) = raw(s, screen, landing)
        // Rolled onto a wing in the sway, and into the swoop's turns.
        let sway = 0.18 * sin(1.1 * s + 0.4) * Ease.smoothstep(Wormhole.jump, Wormhole.jump + 1, s)
            * (1 - Ease.smoothstep(swoop, swoop + 0.3, s))
        let dt: Float = 0.04
        let rate = (raw(s + dt, screen, landing).2 - raw(s - dt, screen, landing).2) / (2 * dt)
        let turning: Float = (swoop..<dock).contains(s) ? -0.5 * rate : 0
        let bank = min(max(sway + turning, -0.6), 0.6)

        // The launch's burn, easing off as the funnel gathers, full in the cruise, cut at the drop out, lit
        // again for the swoop and out as it docks.
        let launch = 1 - 0.45 * Ease.smoothstep(0.6, Wormhole.jump, s)
        let cruise = Ease.smoothstep(Wormhole.jump - 0.05, Wormhole.jump + 0.3, s)
        let cut = Ease.smoothstep(Wormhole.exit - 0.15, Wormhole.exit + 0.1, s)
        let swoopBurn = Ease.smoothstep(swoop, swoop + 0.4, s)
        let docking = 1 - Ease.smoothstep(dock - 0.3, land, s)
        let thrust: Float =
            s < Wormhole.exit ? max(launch, cruise) : (0.25 * (1 - cut) + 0.25 * cut + 0.45 * swoopBurn) * docking
        let flare = exp(-pow((s - Wormhole.jump) / 0.12, 2)) + 0.35 * exp(-pow((s - Wormhole.exit) / 0.15, 2))
        let opacity = 1 - Ease.smoothstep(land + 0.05, land + 0.05 + dissolve, s)
        return Ship(
            pos: pos, size: size, heading: heading, bank: bank, thrust: min(max(thrust, 0), 1), opacity: opacity,
            flare: min(flare, 1))
    }

    /// Where the ship was at the last `trailPoints` steps of `trailStep` seconds, newest first.
    public static func trail(at seconds: Float, screen: SIMD2<Float>, landing: Landing) -> [SIMD2<Float>] {
        (0..<trailPoints).map { ship(at: seconds - Float($0) * trailStep, screen: screen, landing: landing).pos }
    }
}
