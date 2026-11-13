// Which layout change the pane area is playing as a split opening. It sees each layout once, as the body draws it (the frame
// the new layout first appears in must already be the opening's, or the terminals would be sized once for nothing), and asks
// the core whether one pane gave half its room to a new one. Plain state, never observed: the body reads it, a timer asks
// for the frame after the motion ends.

import Foundation
import PanesModel
import WorkspaceModel

@MainActor
final class SplitOpeningTracker {
    /// A layout change that is an opening, and when it began.
    struct Playing: Equatable {
        let opening: SplitOpening
        /// The pane that came.
        let new: PaneID
        let began: Date

        func ended(at now: Date) -> Bool { now.timeIntervalSince(began) >= opening.duration }
    }

    /// What the last layout change was: an opening that plays, one that is cut (Reduce Motion, the Motion feature off),
    /// or neither.
    enum Change {
        case none, cut, plays(Playing)

        var isCut: Bool { if case .cut = self { true } else { false } }
    }

    private var seen: PanesState?
    private var change = Change.none

    /// The change `panes` is: decided once, the first time this layout is drawn.
    func observe(_ panes: PanesState, motion: (any SplitMotion)?, plays: Bool) -> Change {
        guard panes != seen else { return change }
        let before = seen
        seen = panes
        change = .none
        guard let motion, let before, before.zoomed == nil, panes.zoomed == nil else { return change }
        let came = panes.unzoomed.map(\.id).filter { id in !before.unzoomed.contains { $0.id == id } }
        guard came.count == 1, let new = came.first, panes.unzoomed.count == before.unzoomed.count + 1,
            let opening = motion.opening(before: before.unzoomed, after: panes.unzoomed, new: new)
        else { return change }
        change = plays ? .plays(Playing(opening: opening, new: new, began: Date())) : .cut
        return change
    }
}
