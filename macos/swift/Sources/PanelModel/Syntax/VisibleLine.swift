// A line of code made drawable by core's rule (`wormhole_core::view::visible`, through the FFI): tabs as spaces to the next stop and
// what a person cannot see, or what would change how the line reads (a control, a bidirectional override), as a visible mark.
// Offsets are unicode scalars, never Characters: a combining mark after a mark joins it in a Character count and would overrun.

public struct VisibleLine: Sendable, Equatable {
    public var text: String
    /// Where each scalar of the original starts in `text` (one more for its end); empty when nothing moved.
    public var offsets: [Int]
    /// Where the marks are in `text`, in scalars: drawn dimmed.
    public var marks: [Range<Int>]

    public init(text: String, offsets: [Int] = [], marks: [Range<Int>] = []) { (self.text, self.offsets, self.marks) = (text, offsets, marks) }

    /// Where the original's scalar `offset` is in `text`; an offset past the end (a stale highlight range) is the end.
    public func shown(_ offset: Int, total: Int) -> Int {
        guard !offsets.isEmpty else { return min(max(offset, 0), total) }
        return min(offsets[min(max(offset, 0), offsets.count - 1)], total)
    }
}

public enum VisibleRules {
    /// Core's rule; the stand-in of previews and tests leaves the line as it is.
    // why: written once on the main thread at launch, before any window reads it; every later access is a read.
    nonisolated(unsafe) public static var line: @Sendable (_ text: String, _ tabWidth: Int) -> VisibleLine = { text, _ in VisibleLine(text: text) }
}

/// A line as drawn: capped at `limit` characters with an elision mark, coloured by spans in Characters of the original.
public struct DrawnLine: Sendable, Equatable {
    public var text: String
    /// The span ranges, in scalars of `text`, clamped inside it.
    public var spans: [SyntaxSpan]
    public var marks: [Range<Int>]
    public var scalars: Int

    public init(_ line: String, spans: [SyntaxSpan], tabWidth: Int, limit: Int) {
        let cut = line.index(line.startIndex, offsetBy: limit, limitedBy: line.endIndex).flatMap { $0 < line.endIndex ? $0 : nil }
        let original = cut.map { String(line[..<$0]) } ?? line
        let visible = VisibleRules.line(original, tabWidth)
        let drawn = visible.text + (cut == nil ? "" : "\u{2026}")
        let total = drawn.unicodeScalars.count
        (text, scalars) = (drawn, total)
        marks = visible.marks.map { min($0.lowerBound, total)..<min($0.upperBound, total) }
        if cut != nil { marks.append((total - 1)..<total) }
        // Characters of the original -> scalars of the original -> scalars of the drawn text.
        var starts = [0]
        for character in original { starts.append(starts[starts.count - 1] + character.unicodeScalars.count) }
        self.spans = spans.compactMap { span in
            let (low, high) = (span.range.lowerBound, span.range.upperBound)
            guard low >= 0, low < high, low < starts.count else { return nil }
            let start = visible.shown(starts[low], total: total)
            let end = visible.shown(starts[min(high, starts.count - 1)], total: total)
            return start < end ? SyntaxSpan(range: start..<end, key: span.key) : nil
        }
    }
}
