// Where a point in the view is in the grid. The grid sits at the view's top-left corner (the view is
// flipped), one cell `cellWidth` x `cellHeight` points, so this is arithmetic and needs no renderer.

import CoreGraphics
import TerminalCore

struct CellHit: Equatable {
    var row: Int
    var column: Int
    /// The half of the cell the point is over.
    var side: CellSide
    /// Points from the grid's corner, never negative (what a program that asked for pixels is told).
    var x: Int
    var y: Int
}

struct CellGrid: Equatable {
    var cellWidth: CGFloat
    var cellHeight: CGFloat
    var columns: Int
    var rows: Int

    var size: CGSize { CGSize(width: cellWidth * CGFloat(columns), height: cellHeight * CGFloat(rows)) }

    func contains(_ point: CGPoint) -> Bool {
        point.x >= 0 && point.y >= 0 && point.x < size.width && point.y < size.height
    }

    /// The cell under `point`, clamped to the grid: a drag past an edge selects up to the edge cell.
    func hit(_ point: CGPoint) -> CellHit? {
        guard columns > 0, rows > 0, cellWidth > 0, cellHeight > 0 else { return nil }
        let x = max(0, point.x), y = max(0, point.y)
        let column = min(Int(x / cellWidth), columns - 1)
        let row = min(Int(y / cellHeight), rows - 1)
        let inside = (x / cellWidth) - CGFloat(column)
        return CellHit(row: row, column: column, side: inside < 0.5 ? .left : .right, x: Int(x), y: Int(y))
    }

    /// Lines to scroll (up is positive) for a drag at `point`, or `nil` while it is over the grid. The
    /// distance past the top or bottom, in cells (the app's rule: `terminal_view/input.rs` `drag_scroll_lines`);
    /// 0 beside the grid, where the selection only follows.
    func dragScroll(_ point: CGPoint) -> Int? {
        guard rows > 0, cellHeight > 0 else { return nil }
        if point.y < 0 { return Int((-point.y / cellHeight).rounded(.up)) }
        if point.y >= size.height { return -(Int(((point.y - size.height) / cellHeight).rounded(.down)) + 1) }
        if point.x < 0 || point.x >= size.width { return 0 }
        return nil
    }
}
