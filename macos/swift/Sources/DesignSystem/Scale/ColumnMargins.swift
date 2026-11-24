// How far the cards of the side columns and the tab strip stand from the window's edges and from the neighbouring column. With
// the one fill (`one_background`) the window has a single distance for all of it, so no column keeps a margin of its own; with it
// off each column keeps the GPUI app's numbers. The window decides which, once, and hands this value to the columns through the
// environment (the sidebar and the panel cannot see the window's package).

import SwiftUI

/// The distances the window's cards keep from its edges and from each other: one, or each column's own.
public struct ColumnMargins: Sendable, Equatable {
    /// The one distance, or nil while every column keeps its own.
    public let edge: CGFloat?

    /// Which side column a card is in: the terminal's column lies on its other side.
    public enum Column: Sendable { case sidebar, panel }

    public init(edge: CGFloat?) { self.edge = edge }

    /// Each column's own margins.
    public static let own = ColumnMargins(edge: nil)

    /// A card's distance from a column's left and right edge. With the one distance the terminal's card keeps it from the
    /// column that lies beside it, so the card's own margin on that side is none and the gap between the two is one distance
    /// (not two); a slide never shifts it, because the terminal's margins do not change with a column.
    public func sides(own: CGFloat, in column: Column) -> (leading: CGFloat, trailing: CGFloat) {
        guard let edge else { return (own, own) }
        return column == .sidebar ? (edge, 0) : (0, edge)
    }

    /// The room above a column's first card: none with the one distance, since the card then starts on the title row's bottom
    /// as the terminal's does.
    public func top(own: CGFloat) -> CGFloat { edge == nil ? own : 0 }

    /// The room under a column's last card, besides the `gap` every card keeps below itself: what makes the last one end one
    /// distance above the window's (or the status bar's) edge whatever the gap between cards is.
    public func bottom(own: CGFloat, gap: CGFloat) -> CGFloat { edge.map { max($0 - gap, 0) } ?? own }

    /// The one distance, or `own` where a part keeps its own.
    public func value(own: CGFloat) -> CGFloat { edge ?? own }
}

extension EnvironmentValues {
    /// The margins of the column or strip being drawn (`ColumnMargins.own` unless the window sets them).
    @Entry public var columnMargins = ColumnMargins.own
}
