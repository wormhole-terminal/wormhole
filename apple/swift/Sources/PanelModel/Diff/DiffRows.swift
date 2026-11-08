// A diff cut into the rows the view draws (`features/git/diff/rows.rs`): unified, a hunk header then its lines;
// split, the old side against the new with a filler where one side has no line (core's `git::split_rows`).

public struct DiffCell: Hashable, Sendable {
    public var kind: DiffLine.Kind
    public var old: Int?
    public var new: Int?
    public var text: String
    public var noNewline = false
    public var spans: [SyntaxSpan] = []
}

public enum DiffRow: Hashable, Sendable {
    case hunk(String)
    case unified(DiffCell)
    /// Old side, new side; nil is a filler.
    case split(DiffCell?, DiffCell?)
}

public enum DiffRows {
    public static func unified(_ file: DiffFile) -> [DiffRow] {
        file.hunks.flatMap { [DiffRow.hunk($0.header)] + $0.lines.map(cell).map(DiffRow.unified) }
    }

    /// Runs of removed then added lines pair up line by line; the longer run's rest meets fillers (core's `git::split_rows`).
    public static func split(_ file: DiffFile) -> [DiffRow] {
        var rows: [DiffRow] = []
        for hunk in file.hunks {
            rows.append(.hunk(hunk.header))
            var removed: [DiffCell] = [], added: [DiffCell] = []
            func flush() {
                for index in 0..<max(removed.count, added.count) {
                    rows.append(.split(index < removed.count ? removed[index] : nil, index < added.count ? added[index] : nil))
                }
                removed = []
                added = []
            }
            for cell in hunk.lines.map(cell) {
                switch cell.kind {
                case .removed:
                    // A removal after additions starts the next change block, as in core's `split_rows`.
                    if !added.isEmpty { flush() }
                    removed.append(cell)
                case .added: added.append(cell)
                default:
                    flush()
                    rows.append(.split(cell, cell))
                }
            }
            flush()
        }
        return rows
    }

    /// Digits in the largest line number, at least 3 so the gutter does not jump between files.
    public static func numberDigits(_ file: DiffFile) -> Int {
        let largest = file.hunks.flatMap(\.lines).compactMap { max($0.oldNumber ?? 0, $0.newNumber ?? 0) }.max() ?? 0
        return max(String(largest).count, 3)
    }

    private static func cell(_ line: DiffLine) -> DiffCell {
        DiffCell(kind: line.kind, old: line.oldNumber, new: line.newNumber, text: line.text, noNewline: line.noNewline, spans: line.spans)
    }
}
