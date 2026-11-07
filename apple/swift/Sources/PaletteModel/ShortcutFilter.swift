// What the sheet's filter finds (core `sheet::matches` and `filter`): text in a label or in the keys as written
// ("f6"); a query with a modifier glyph is a key, and finds the rows that have that very key ("⌘W" is Close
// terminal, not Close window's ⇧⌘W), also inside a range.

import Foundation

public struct ShortcutHit: Sendable, Equatable {
    /// Where the label matched, as `Character` offsets, to mark.
    public var label: Range<Int>?
}

public struct FilteredShortcuts: Sendable, Equatable {
    public var sections: [(title: String, rows: [(row: ShortcutRow, hit: ShortcutHit)])]
    public var shown: Int
    public var total: Int

    public static func == (a: Self, b: Self) -> Bool {
        a.shown == b.shown && a.total == b.total && a.sections.map(\.title) == b.sections.map(\.title)
            && a.sections.flatMap { $0.rows.map(\.row) } == b.sections.flatMap { $0.rows.map(\.row) }
    }
}

extension ShortcutSheet {
    public static func matches(_ query: String, in row: ShortcutRow) -> ShortcutHit? {
        let query = query.trimmingCharacters(in: .whitespaces)
        if query.isEmpty { return ShortcutHit(label: nil) }
        if query.contains(where: { "⌘⌥⌃⇧".contains($0) }) {
            guard let wanted = Shortcut(glyphs: query) else {
                return row.allKeys.contains { $0.glyphs.contains(query) } ? ShortcutHit(label: nil) : nil
            }
            return row.allKeys.contains(wanted) ? ShortcutHit(label: nil) : nil
        }
        let (label, wanted) = (row.label.lowercased(), query.lowercased())
        if let found = label.range(of: wanted) {
            let start = label.distance(from: label.startIndex, to: found.lowerBound)
            return ShortcutHit(label: start..<(start + wanted.count))
        }
        return row.allKeys.contains { $0.glyphs.lowercased().contains(wanted) } ? ShortcutHit(label: nil) : nil
    }

    public static func filter(_ query: String, in sections: [ShortcutSection] = sections) -> FilteredShortcuts {
        let found = sections.compactMap { section -> (String, [(ShortcutRow, ShortcutHit)])? in
            let rows = section.rows.compactMap { row in matches(query, in: row).map { (row, $0) } }
            return rows.isEmpty ? nil : (section.title, rows)
        }
        return FilteredShortcuts(
            sections: found.map { (title: $0.0, rows: $0.1.map { (row: $0.0, hit: $0.1) }) },
            shown: found.reduce(0) { $0 + $1.1.count }, total: sections.reduce(0) { $0 + $1.rows.count })
    }
}
