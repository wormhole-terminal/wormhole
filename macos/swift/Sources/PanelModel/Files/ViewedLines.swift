// A file's text cut into the lines the viewer numbers, as the GPUI viewer cuts them (`features/files/viewer/lines.rs`).

public enum ViewedLines {
    /// Cut at every line feed; a carriage return before it is not part of the line, and a last line break adds no
    /// line. Scalars, not Characters: Swift reads "\r\n" as one Character, which never equals "\n".
    public static func cut(_ text: String) -> [String] {
        var lines = text.unicodeScalars.split(separator: "\n", omittingEmptySubsequences: false).map { line in
            String(String.UnicodeScalarView(line.last == "\r" ? line.dropLast() : line))
        }
        if lines.count > 1, lines.last == "" { lines.removeLast() }
        return lines
    }
}
