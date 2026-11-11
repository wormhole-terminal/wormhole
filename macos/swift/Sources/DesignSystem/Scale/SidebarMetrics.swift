// What the sidebar's rows, cards and header measure at each density, with the cards on or off: a port of
// `crates/wormhole/src/style/metrics.rs` `Metrics::of` (RF-10, RF-40), the one place for those numbers.

import SwiftUI

public enum SidebarDensity: String, Sendable, CaseIterable {
    case regular, compact, dense

    /// The row's second line (what the terminal is doing, or where it is) is drawn.
    public var twoLines: Bool { self == .regular }
}

public struct SidebarMetrics: Sendable, Equatable {
    public let density: SidebarDensity
    public let cards: Bool
    /// A terminal's row: its height, its status tile and the gap after it.
    public let rowHeight: CGFloat
    public let tile: CGFloat
    public let tileGap: CGFloat
    /// The row's padding at its sides, and the corner of its selection and hover.
    public let rowInset: CGFloat
    public let rowRadius: CGFloat
    /// The card's distance from the sidebar's sides, its padding, its corner and the gap under it.
    public let cardMargin: CGFloat
    public let cardPad: CGFloat
    public let cardRadius: CGFloat
    public let cardGap: CGFloat
    /// A group's head, its height with a second line for the folder, and the tiles it keeps while folded.
    public let headHeight: CGFloat
    public let headTwoLineHeight: CGFloat
    public let foldedTile: CGFloat
    public let foldedGap: CGFloat
    /// A pinned group with nothing in it.
    public let emptyHeight: CGFloat
    /// The "Terminals" header: its height, how far in it stands, the + button and whether the title is the small one.
    public let headerHeight: CGFloat
    public let headerInset: CGFloat
    public let addButton: CGFloat
    public let smallTitle: Bool

    public init(density: SidebarDensity = .regular, cards: Bool = true) {
        let dense = density == .dense
        self.density = density
        self.cards = cards
        (rowHeight, tile, tileGap) =
            switch density {
            case .regular: (48, Metric.tile, 10)
            case .compact: (32, Metric.tileMini, 8)
            case .dense: (26, Metric.tileSmall, 8)
            }
        rowInset = dense ? 4 : 8
        rowRadius = dense ? 6 : 8
        (cardMargin, cardPad) =
            switch (dense, cards) {
            case (false, true): (10, 4)
            case (true, true): (4, 2)
            case (false, false): (6, 0)
            case (true, false): (4, 0)
            }
        cardRadius = dense ? 8 : 12
        cardGap = dense ? 4 : 8
        headHeight = dense ? 26 : 36
        headTwoLineHeight = [.regular: 44, .compact: 40, .dense: 26][density] ?? 44
        foldedTile = dense ? 14 : Metric.tileMini
        foldedGap = 4
        emptyHeight = dense ? 24 : 32
        headerHeight = dense ? 22 : 28
        headerInset = dense ? 8 : 12
        addButton = dense ? 20 : 24
        smallTitle = dense
    }

    /// Where a terminal row's first line of text starts, from the row's left edge.
    public var titleX: CGFloat { rowInset + tile + tileGap }
}
