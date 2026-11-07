// Review comments on a diff and the message they make for the coding agent in the terminal (`wormhole_core::git::
// review`, whose rules the FFI will bring): a comment is pinned to a line, comments gather per terminal across
// files, and `message` names each place, quotes the line and says what the person wrote. The quoted line comes from
// a repository that may be anyone's, so the message carries no control characters at all.

public enum LineSide: Sendable { case old, new }

public struct ReviewComment: Hashable, Sendable {
    public var path: String
    public var side: LineSide
    public var line: Int
    /// The line as the diff showed it, quoted in the message.
    public var code: String
    public var text: String

    public init(path: String, side: LineSide, line: Int, code: String, text: String) {
        (self.path, self.side, self.line, self.code, self.text) = (path, side, line, code, text)
    }

    /// "src/orders.rs:42", and for a removed line, which the file no longer has, "src/orders.rs:40 (removed line)".
    public var location: String {
        let place = "\(Review.printable(path, lines: false)):\(line)"
        return side == .new ? place : place + " (removed line)"
    }
}

public struct Review: Equatable, Sendable {
    public private(set) var comments: [ReviewComment] = []
    static let quoteMax = 160

    public init() {}

    public var isEmpty: Bool { comments.isEmpty }

    /// Adds `comment`, or replaces the one on the same line; an empty text removes that one.
    public mutating func put(_ comment: ReviewComment) {
        let at = comments.firstIndex { $0.path == comment.path && $0.side == comment.side && $0.line == comment.line }
        let empty = comment.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        switch (at, empty) {
        case let (index?, true): comments.remove(at: index)
        case let (index?, false): comments[index] = comment
        case (nil, true): break
        case (nil, false): comments.append(comment)
        }
    }

    public func on(_ path: String, _ side: LineSide, _ line: Int) -> ReviewComment? {
        comments.first { $0.path == path && $0.side == side && $0.line == line }
    }

    public mutating func clear() { comments.removeAll() }

    /// "1 comment", "3 comments on 2 files".
    public var summary: String {
        let files = Set(comments.map(\.path)).count
        let count = comments.count == 1 ? "1 comment" : "\(comments.count) comments"
        return files > 1 ? "\(count) on \(files) files" : count
    }

    /// The message for the agent: a line saying what follows, then each comment numbered with its place, the quoted
    /// line and its text. Nil without comments.
    public var message: String? {
        guard !comments.isEmpty else { return nil }
        var out = "Review comments on your changes. Please address each one:\n"
        for (index, comment) in comments.enumerated() {
            out += "\n\(index + 1). \(comment.location)\n"
            let quote = Self.quote(comment.code)
            if !quote.isEmpty { out += "   > \(quote)\n" }
            let lines = Self.printable(comment.text, lines: true).trimmingCharacters(in: .whitespacesAndNewlines)
            for line in lines.split(separator: "\n", omittingEmptySubsequences: false) {
                out += "   " + line.replacing(/\s+$/, with: "") + "\n"
            }
        }
        return out.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func quote(_ code: String) -> String {
        let code = printable(code, lines: false).trimmingCharacters(in: .whitespaces)
        guard code.count > quoteMax else { return code }
        return code.prefix(quoteMax).trimmingCharacters(in: .whitespaces) + "\u{2026}"
    }

    /// `text` without control characters (escape sequences, the bell, a carriage return a terminal would act on); a
    /// tab becomes a space, and line breaks stay only in a comment's own text.
    static func printable(_ text: String, lines: Bool) -> String {
        String(
            text.unicodeScalars.compactMap { scalar in
                switch scalar {
                case "\n" where lines: "\n"
                case "\t": " "
                default: scalar.properties.generalCategory == .control ? nil : scalar
                }
            }.map(Character.init))
    }
}
