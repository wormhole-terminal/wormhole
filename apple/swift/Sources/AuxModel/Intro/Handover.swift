// How the wormhole run hands over to the terminal welcome (`Wormhole welcome`, a program in the first terminal)
// instead of the intro window's page (`onboarding::intro::handover`). All three are the Voyage run; they differ in
// whether the ship flies, where it lands, and when the program is told to start.

import Foundation

public enum Handover: String, CaseIterable, Sendable {
    /// The ship docks on the terminal cell where the program draws its `>_` mark. The first launch uses it.
    case cursor
    /// No ship: the text types as the light lifts away.
    case portal
    /// The ship docks on the chevron of a block-art logo.
    case logo

    public var label: String { rawValue.capitalized }
    public var style: IntroStyle { .voyage }
    public var ship: Bool { self != .portal }

    /// When the program's text should start, in seconds into the run.
    public var textStarts: Float { self == .portal ? Voyage.lift : Voyage.land }

    /// How long the program waits before it draws, `elapsed` seconds into a run that is playing. Nothing to wait
    /// for when no run plays or under Reduce Motion, where the glow stands in and nothing flies.
    public func wait(elapsed: Float?, reduceMotion: Bool) -> Float {
        guard let elapsed, !reduceMotion else { return 0 }
        return max(textStarts - elapsed, 0)
    }
}
