// How the intro window's page plays into the animation: when each part enters once the window is on screen, and
// how the traffic lights go (`onboarding::intro::page`). The page is what the window carries through the
// wormhole, so it begins to enter just as the window arrives, or as the ship docks on the logo in Voyage.

import Foundation

/// How far each part of the page has entered, 0 to 1: they rise and fade in one after another.
public struct Entrance: Equatable, Sendable {
    public var logo, title, tagline, body, actions, corner: Float

    public static let done = Entrance(logo: 1, title: 1, tagline: 1, body: 1, actions: 1, corner: 1)
    public static let none = Entrance(logo: 0, title: 0, tagline: 0, body: 0, actions: 0, corner: 0)

    /// The least of the parts: 1 only when all of them are in.
    public var least: Float { min(logo, title, tagline, body, actions, corner) }
}

extension IntroStyle {
    /// When the window is on screen, in seconds into the run.
    public var windowArrives: Float {
        switch self {
        case .emerge: 0.5 * seconds
        case .collapse: 0.6 * seconds
        case .ftl: Self.ftlExit * seconds
        case .aurora, .tentacles, .voyage: Wormhole.arrive
        case .vortex, .glow: 0
        }
    }

    /// A beat after the window is there before the first part moves; in Voyage as the ship docks.
    private var entersFrom: Float { self == .voyage ? Voyage.land - 0.15 : windowArrives + 0.1 }
    private static let delays: [Float] = [0, 0.12, 0.24, 0.42, 0.62, 0.85]
    private static let rise: Float = 0.6

    /// By when, in seconds into the run, every part of the page is in. Under Reduce Motion nothing moves.
    public var enteredBy: Float { self == .glow ? 0 : entersFrom + Self.delays[5] + Self.rise }

    /// The page's parts at `seconds` into the run.
    public func entrance(at seconds: Float) -> Entrance {
        if self == .glow { return .done }
        let part = { (delay: Float) in
            Ease.outCubic(Ease.phase(seconds, entersFrom + delay, entersFrom + delay + Self.rise))
        }
        let d = Self.delays
        return Entrance(logo: part(d[0]), title: part(d[1]), tagline: part(d[2]), body: part(d[3]), actions: part(d[4]), corner: part(d[5]))
    }

    /// The traffic lights' opacity at `seconds`: they fade away as a replay begins and stay away.
    public static func lights(at seconds: Float, start: IntroStart) -> Float {
        start == .shown ? 1 - Ease.smoothstep(0, 0.25, seconds) : 0
    }
}
