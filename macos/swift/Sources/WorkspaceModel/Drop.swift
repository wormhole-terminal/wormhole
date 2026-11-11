// A drag in the list, in the core's terms (core `workspace::Over` and `DropPlan`): the view hit-tests what the pointer
// is over, by id, and the core says what letting go would do; the view only draws the line for the answer.

import Foundation

/// Which part of a line the pointer is over (core `Part`): the top quarter reorders before it, the bottom quarter
/// after it, and the middle half joins a split (on a head: joins the group).
public enum DropPart: Sendable, Equatable {
    case top, middle, bottom

    public init(fraction: Double) { self = fraction < 0.25 ? .top : fraction > 0.75 ? .bottom : .middle }
}

/// The card a gap in the list sits under.
public enum CardRef: Sendable, Equatable, Hashable {
    case group(GroupID)
    /// A terminal in no group.
    case loose(TerminalID)
    /// The terminals of a split in no group, named by the first of them.
    case split(TerminalID)
}

public enum DropOver: Sendable, Equatable {
    case row(TerminalID, DropPart)
    case head(GroupID, DropPart)
    case after(CardRef)
    /// The space under the last line.
    case tail
}

/// One thing at the top level of the list.
public enum TopItem: Sendable, Equatable, Hashable {
    case terminal(TerminalID)
    case group(GroupID)
}

/// A place in the list: at the top level before an item (last without one), or in a group before one of its
/// terminals (last without one).
public enum Landing: Sendable, Equatable {
    case top(before: TopItem?)
    case group(GroupID, before: TerminalID?)
}

public enum DropPlan: Sendable, Equatable {
    /// It moves to `landing` (with the rest of its split when `withMates`).
    case move(Landing, leavesSplit: Bool, withMates: Bool)
    /// It makes a split with this terminal, or joins its split.
    case join(TerminalID)
}
