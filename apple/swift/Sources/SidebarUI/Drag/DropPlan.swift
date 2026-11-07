// What a drop does and how it shows: the action for the model and the mark that says where it lands (a line in the gap
// between two cards or two rows, or a group's card outlined when the drop joins it).

import CoreGraphics
import WorkspaceModel

struct DropPlan: Equatable {
    enum Mark: Equatable {
        /// A line across `x` at `y`.
        case line(y: CGFloat, x: ClosedRange<CGFloat>)
        /// The group's card.
        case card(String)
    }

    var action: WorkspaceAction
    var mark: Mark
}

/// Which part of a line the pointer is over (`Part` in `view/sidebar/drop.rs`): the top quarter reorders before it, the
/// bottom quarter after it, and the middle half joins a split. The model has no join yet, so a terminal's middle half
/// goes to the nearer edge.
enum DropPart {
    case top, middle, bottom

    init(_ fraction: CGFloat) { self = fraction < 0.25 ? .top : fraction > 0.75 ? .bottom : .middle }
}

enum DropGeometry {
    /// How much of a group head's height is "before the group": its top quarter.
    static let headBefore: CGFloat = 0.25
}
