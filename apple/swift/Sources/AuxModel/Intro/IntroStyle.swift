// The wormhole effect that plays over the whole screen around a window the first time Wormhole opens, and
// again from About: seven styles, plus the still glow that stands in for all of them under Reduce Motion.
// Core owns this (`onboarding::intro::Style`); the Rust FFI will replace this port, which keeps the same
// names, keys and numbers so the shared shader reads them unchanged.

import Foundation

public enum IntroStyle: String, CaseIterable, Sendable {
    case emerge, vortex, collapse, ftl, aurora, tentacles, voyage, glow

    /// The styles that move, in the order About offers them.
    public static let onTrial: [IntroStyle] = [.emerge, .vortex, .collapse, .ftl, .aurora, .tentacles, .voyage]

    /// The style the intro window plays, first launch and replay alike: the one whose ship docks on its logo.
    public static let firstLaunch = IntroStyle.voyage

    /// What plays: under Reduce Motion, the glow whatever was asked.
    public func withMotion(reduceMotion: Bool) -> IntroStyle { reduceMotion ? .glow : self }

    public var label: String {
        switch self {
        case .emerge: "Emerge"
        case .vortex: "Edge Vortex"
        case .collapse: "Collapse"
        case .ftl: "FTL Jump"
        case .aurora: "Aurora"
        case .tentacles: "Tentacles"
        case .voyage: "Voyage"
        case .glow: "Glow"
        }
    }

    public var seconds: Float {
        switch self {
        case .emerge: 4.2
        case .vortex: 3.4
        case .collapse: 4.6
        case .ftl: 7.0
        case .aurora, .tentacles: Wormhole.seconds
        case .voyage: Voyage.seconds
        case .glow: 1.2
        }
    }

    /// The shader's number for it (`entry.metal`'s `switch`).
    public var shaderID: UInt32 {
        switch self {
        case .emerge: 0
        case .vortex: 1
        case .collapse: 2
        case .glow: 3
        case .ftl: 4
        case .aurora: 5
        case .tentacles: 6
        case .voyage: 7
        }
    }

    /// How the wormhole styles lay out their run for the shader.
    public var run: IntroRun {
        switch self {
        // The ship docks, and the arms bloom from there, a beat after.
        case .voyage:
            IntroRun(seconds: Voyage.seconds, lift: Voyage.lift, bloom: Voyage.land - Wormhole.exit - 0.1, arms: 2, mist: 1)
        default:
            IntroRun(seconds: Wormhole.seconds, lift: Wormhole.lift, bloom: 0.1, arms: 1, mist: 0)
        }
    }
}

/// How a wormhole style's run is laid out, in seconds: how long it is, when the light behind the window begins to
/// rise away and fade, when it blooms after the drop out, how big the arms of light are (1 is Tentacles'), and how
/// misty it is (1 swaps the funnel's glitter for the plasma tunnel and blurs the arms).
public struct IntroRun: Equatable, Sendable {
    public var seconds, lift, bloom, arms, mist: Float
}
