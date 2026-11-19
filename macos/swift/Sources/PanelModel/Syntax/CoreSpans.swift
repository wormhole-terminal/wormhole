// Spans as the core makes them (tree-sitter, over the FFI) count a line in unicode scalars, which no line a repository holds
// can make it miscount; the views' spans count Characters. This is the one place the two meet.

extension SyntaxSpan {
    /// `runs`, in scalars of `line`, as spans in Characters of it. A run that ends inside a Character takes the whole of it;
    /// one that leaves the line is dropped.
    public static func inCharacters(_ runs: [(range: Range<Int>, key: String)], of line: String) -> [SyntaxSpan] {
        guard !runs.isEmpty else { return [] }
        // The Character every scalar is in.
        var owner: [Int] = []
        owner.reserveCapacity(line.unicodeScalars.count)
        for (index, character) in line.enumerated() {
            owner.append(contentsOf: repeatElement(index, count: character.unicodeScalars.count))
        }
        return runs.compactMap { run in
            guard run.range.lowerBound >= 0, run.range.lowerBound < run.range.upperBound, run.range.upperBound <= owner.count else { return nil }
            return SyntaxSpan(range: owner[run.range.lowerBound]..<(owner[run.range.upperBound - 1] + 1), key: run.key)
        }
    }
}
