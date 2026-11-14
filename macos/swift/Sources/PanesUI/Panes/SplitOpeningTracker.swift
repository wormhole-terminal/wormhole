// Which layout change the pane area is playing as a split opening. It sees each layout once, as the body draws it (the frame
// the new layout first appears in must already be the opening's, or the terminals would be sized once for nothing). Only the
// pane a split opened plays (the window says which: GPUI's `split_pane` asks for the opening, a drop or a join does not), and
// the core says how one pane gives half its room to it. Plain state, never observed: the body reads it, a timer asks for
// the frame after the motion ends.

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
    /// The split's pane the last layout change was weighed against: a split opens one layout change at most.
    private var weighed: PaneID?

    /// The change `panes` is: decided once, the first time this layout is drawn. `split`: the terminal the window's last split
    /// opened.
    func observe(_ panes: PanesState, motion: (any SplitMotion)?, plays: Bool, split: PaneID?) -> Change {
        guard panes != seen else { return change }
        let before = seen
        seen = panes
        change = .none
        // The same pane coming again later (it left the split and was dropped back) is a drop, not this split.
        let asked = split == weighed ? nil : split
        weighed = split
        guard let motion, let before, before.zoomed == nil, panes.zoomed == nil else { return change }
        let came = panes.unzoomed.map(\.id).filter { id in !before.unzoomed.contains { $0.id == id } }
        guard came.count == 1, let new = came.first, new == asked, panes.unzoomed.count == before.unzoomed.count + 1,
            let opening = motion.opening(before: before.unzoomed, after: panes.unzoomed, new: new)
        else { return change }
        change = plays ? .plays(Playing(opening: opening, new: new, began: Date())) : .cut
        return change
    }
}
