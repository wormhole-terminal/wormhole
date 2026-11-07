// What the view marks on a row beyond what the frame says: the selected columns, find's matches and the hovered
// link's. They are applied to a copy of the row's cells before the builder sees it, so the builder's hot loop
// (every row of every frame) never learns of them and costs nothing when nothing is marked.

import TerminalCore

struct FoundColumns: Equatable {
    var first: Int
    var last: Int
    var isCurrent: Bool
}

struct RowMarks: Equatable {
    var selected: ClosedRange<Int>?
    var link: ClosedRange<Int>?
    /// In column order, as the search found them.
    var found: [FoundColumns] = []
    static let none = RowMarks()

    var isMarked: Bool { selected != nil || link != nil || !found.isEmpty }

    /// A cell's colours as drawn with inverse resolved and hidden text kept hidden: the base a mark paints over.
    private static func normalised(_ cell: inout PackedCell) {
        var fg = cell.fg, bg = cell.bg
        let flags = cell.cellFlags
        if flags.contains(.inverse) { swap(&fg, &bg) }
        if flags.contains(.hidden) { fg = bg }
        cell.fg = fg
        cell.bg = bg
        cell.flags = flags.subtracting([.inverse, .hidden]).rawValue
    }

    /// A selected cell is drawn as the selection's colours over the cell as it would have been drawn; without a
    /// theme text colour the text keeps its own. Find's current match stands out over everything, in bold; the
    /// others tint the background, except inside the selection (`render/rows.rs`). A hovered link is underlined,
    /// whatever the program drew, in the text's colour.
    func apply(to cells: inout [PackedCell], palette: TerminalPalette) {
        if let selected {
            for column in selected where column < cells.count {
                let wasHidden = cells[column].cellFlags.contains(.hidden)
                Self.normalised(&cells[column])
                if !palette.keepsTextInSelection, !wasHidden { cells[column].fg = PackedColor.selectionForeground }
                cells[column].bg = PackedColor.selectionBackground
            }
        }
        for match in found {
            for column in match.first...match.last where column < cells.count {
                if match.isCurrent {
                    Self.normalised(&cells[column])
                    cells[column].fg = PackedColor.findCurrentText
                    cells[column].bg = PackedColor.findCurrent
                    cells[column].flags |= CellFlags.bold.rawValue
                } else if selected?.contains(column) != true {
                    Self.normalised(&cells[column])
                    cells[column].bg = palette.tinted(cells[column].bg)
                }
            }
        }
        if let link {
            for column in link where column < cells.count && cells[column].cellFlags.isDisjoint(with: .allUnderlines) {
                cells[column].flags |= CellFlags.underline.rawValue
            }
        }
    }
}
