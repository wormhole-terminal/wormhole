// The list as drawn: results, with a heading before each group while nothing is typed (typing ranks across
// groups, so headings would lie).

import DesignSystem
import PaletteModel
import SwiftUI

enum PaletteItem: Identifiable {
    case header(String)
    case result(PaletteResult)

    static let rowHeight = Space._4 * 2 + Space._2
    static let maxListHeight: CGFloat = 360

    var id: String {
        switch self {
        case .header(let title): "header-\(title)"
        case .result(let result): result.id
        }
    }

    static func items(for results: [PaletteResult], grouped: Bool) -> [PaletteItem] {
        guard grouped else { return results.map(PaletteItem.result) }
        var out: [PaletteItem] = []
        var group: PaletteGroup?
        for result in results {
            if result.command.group != group {
                group = result.command.group
                out.append(.header(result.command.group.title))
            }
            out.append(.result(result))
        }
        return out
    }

    /// The list sizes to its rows up to a cap, so the card grows and shrinks with the results.
    static func height(of items: [PaletteItem]) -> CGFloat {
        let content = items.reduce(Space._4) { total, item in
            switch item {
            case .header: total + Space._6 + Space._1
            case .result: total + PaletteItem.rowHeight
            }
        }
        return min(content, maxListHeight)
    }
}
