// Where the picker's card goes (`view/picker/place.rs`): against the terminal's cursor, above its line, and
// inside the pane. Pure, so the tight places (a cursor on the first line, a small pane) are tested.

import CoreGraphics
import Foundation

public struct PickerCard: Sendable, Equatable {
    public var width: Double
    /// What it has besides rows: its padding, the field, the hint line and the breadcrumb.
    public var chrome: Double
    public var row: Double
    public var maxRows: Int

    public init(width: Double, chrome: Double, row: Double, maxRows: Int) { (self.width, self.chrome, self.row, self.maxRows) = (width, chrome, row, maxRows) }

    public func height(rows: Int) -> Double { chrome + row * Double(rows) }
}

public struct PickerPlaced: Sendable, Equatable {
    public var rect: CGRect
    /// How many rows the card has room for: at most what was wanted, fewer in a pane too short for all.
    public var rows: Int
    /// The card is above the cursor's line; below it when there was no room above.
    public var above: Bool
}

public enum PickerPlacement {
    /// The room kept between the card and the cursor's cell, and between the card and the pane's edges.
    public static let gap = 4.0, margin = 8.0

    /// The side is decided by whether the card at its largest fits, not by how many rows there are now, so typing,
    /// which makes the list shorter, never moves the card across the line. Top-left origin, as the pane.
    public static func place(pane: CGRect, cursor: CGRect, card: PickerCard, wanted: Int) -> PickerPlaced {
        let top = pane.minY + margin, bottom = pane.maxY - margin
        let roomAbove = max(0, cursor.minY - gap - top)
        let roomBelow = max(0, bottom - (cursor.maxY + gap))
        let full = card.height(rows: card.maxRows)
        let above = roomAbove >= full || (roomBelow < full && roomAbove >= roomBelow)
        let room = above ? roomAbove : roomBelow
        let fits = max(1, Int(((room - card.chrome) / card.row).rounded(.down)))
        let rows = min(min(max(wanted, 1), card.maxRows), fits)
        let height = card.height(rows: rows)
        var y = above ? cursor.minY - gap - height : cursor.maxY + gap
        y = max(min(y, bottom - height), top)
        let width = min(card.width, max(0, pane.width - 2 * margin))
        let x = max(min(cursor.minX, pane.maxX - margin - width), pane.minX + margin)
        return PickerPlaced(rect: CGRect(x: x, y: y, width: width, height: height), rows: rows, above: above)
    }
}
