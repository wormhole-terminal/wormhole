// What the view marks on a row beyond what the frame says: the selected columns and the hovered link's. They
// are applied to a copy of the row's cells before the builder sees it, so the builder's hot loop (every row of
// every frame) never learns of them and costs nothing when nothing is marked.

import TerminalCore

struct RowMarks: Equatable {
    var selected: ClosedRange<Int>?
    var link: ClosedRange<Int>?
    static let none = RowMarks()

    var isMarked: Bool { selected != nil || link != nil }

    /// A selected cell is drawn as the selection's colours over the cell as it would have been drawn (inverse
    /// resolved, hidden text kept hidden); without a theme text colour the text keeps its own. A hovered link
    /// is underlined, whatever the program drew, in the text's colour.
    func apply(to cells: inout [PackedCell], keepsTextInSelection: Bool) {
        if let selected {
            for column in selected where column < cells.count {
                var cell = cells[column]
                var flags = cell.cellFlags
                var fg = cell.fg, bg = cell.bg
                if flags.contains(.inverse) { swap(&fg, &bg) }
                if flags.contains(.hidden) { fg = bg }
                flags.subtract([.inverse, .hidden])
                if !keepsTextInSelection, !cell.cellFlags.contains(.hidden) { fg = PackedColor.selectionForeground }
                cell.fg = fg
                cell.bg = PackedColor.selectionBackground
                cell.flags = flags.rawValue
                cells[column] = cell
            }
        }
        if let link {
            for column in link where column < cells.count && cells[column].cellFlags.isDisjoint(with: .allUnderlines) {
                cells[column].flags |= CellFlags.underline.rawValue
            }
        }
    }
}
