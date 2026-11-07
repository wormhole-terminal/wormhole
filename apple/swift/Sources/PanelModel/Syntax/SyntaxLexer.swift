// Cuts lines of code into runs, each named by the syntax key the theme colours (`keyword`, `string`, ...;
// `wormhole_core::theme::tokens` `syntax.*`). Characters between runs are plain and take the text colour. The state
// a line ends in (inside a block comment, inside a triple-quoted string) is the next line's start.

public struct SyntaxSpan: Hashable, Sendable {
    public var range: Range<Int>
    public var key: String

    public init(range: Range<Int>, key: String) {
        self.range = range
        self.key = key
    }
}

/// What a line ends inside of.
public enum SyntaxState: Equatable, Sendable {
    case plain
    case blockComment
    case tripleString(Character)
}

public enum SyntaxLexer {
    /// One line on its own, as a diff's single line is read.
    public static func spans(in line: String, language: SyntaxLanguage) -> [SyntaxSpan] {
        var state = SyntaxState.plain
        return spans(in: line, language: language, state: &state)
    }

    public static func spans(in line: String, language: SyntaxLanguage, state: inout SyntaxState) -> [SyntaxSpan] {
        let chars = Array(line)
        var spans: [SyntaxSpan] = []
        var i = 0
        func add(_ end: Int, _ key: String) {
            spans.append(SyntaxSpan(range: i..<end, key: key))
            i = end
        }
        // A line that starts inside a block comment or string runs on until it closes.
        switch state {
        case .plain: break
        case .blockComment:
            let close = language.blockComment?.close ?? "*/"
            if let end = find(close, from: 0, in: chars) { add(end + close.count, "comment"); state = .plain } else { add(chars.count, "comment") }
        case let .tripleString(quote):
            if let end = find(String(repeating: quote, count: 3), from: 0, in: chars) { add(end + 3, "string"); state = .plain } else { add(chars.count, "string") }
        }
        while i < chars.count {
            let c = chars[i]
            if let comment = language.lineComment, starts(with: comment, at: i, in: chars) {
                add(chars.count, "comment")
            } else if let block = language.blockComment, starts(with: block.open, at: i, in: chars) {
                if let end = find(block.close, from: i + block.open.count, in: chars) { add(end + block.close.count, "comment") } else { add(chars.count, "comment"); state = .blockComment }
            } else if language.tripleQuotes, (c == "\"" || c == "'"), starts(with: String(repeating: c, count: 3), at: i, in: chars) {
                if let end = find(String(repeating: c, count: 3), from: i + 3, in: chars) { add(end + 3, "string") } else { add(chars.count, "string"); state = .tripleString(c) }
            } else if c == "\"" || (c == "'" && language.singleQuotes) || c == "`" && language.singleQuotes {
                add(end(ofStringAt: i, in: chars), "string")
            } else if c == "'", language.rust {
                // A char literal is closed within a few characters; otherwise it is a lifetime.
                let closes = (i + 2 < chars.count && chars[i + 2] == "'") || (i + 3 < chars.count && chars[i + 1] == "\\" && chars[i + 3] == "'")
                add(closes ? end(ofStringAt: i, in: chars) : identifierEnd(chars, from: i + 1), closes ? "string" : "attribute")
            } else if c.isNumber {
                add(numberEnd(chars, from: i), "number")
            } else if c.isLetter || c == "_" {
                let (end, key) = word(chars, from: i, language: language)
                if let key { add(end, key) } else { i = end }
            } else if "+-*/%=<>!&|^~?:".contains(c) {
                add(i + 1, "operator")
            } else if "{}()[];,.".contains(c) {
                add(i + 1, "punctuation")
            } else {
                i += 1
            }
        }
        return spans
    }

    /// The identifier or keyword at `start`: where it ends and its key, nil for a plain word.
    private static func word(_ chars: [Character], from start: Int, language: SyntaxLanguage) -> (end: Int, key: String?) {
        let end = identifierEnd(chars, from: start)
        let text = String(chars[start..<end])
        let next = end < chars.count ? chars[end] : " "
        let afterDot = start > 0 && chars[start - 1] == "."
        if language.keywords.contains(text) { return (end, "keyword") }
        if language.constants.contains(text) { return (end, "boolean") }
        if language.rust, next == "!" { return (end + 1, "function.macro") }
        if next == "(" { return (end, afterDot ? "function.method" : "function") }
        if afterDot { return (end, "property") }
        if chars[start].isUppercase { return (end, "type") }
        return (end, nil)
    }

    private static func starts(with prefix: String, at index: Int, in chars: [Character]) -> Bool {
        let wanted = Array(prefix)
        return index + wanted.count <= chars.count && Array(chars[index..<index + wanted.count]) == wanted
    }

    private static func find(_ needle: String, from start: Int, in chars: [Character]) -> Int? {
        var i = start
        while i + needle.count <= chars.count {
            if starts(with: needle, at: i, in: chars) { return i }
            i += 1
        }
        return nil
    }

    private static func end(ofStringAt start: Int, in chars: [Character]) -> Int {
        var i = start + 1
        while i < chars.count {
            if chars[i] == "\\" { i += 2; continue }
            if chars[i] == chars[start] { return i + 1 }
            i += 1
        }
        return chars.count
    }

    private static func identifierEnd(_ chars: [Character], from start: Int) -> Int {
        var i = start
        while i < chars.count, chars[i].isLetter || chars[i].isNumber || chars[i] == "_" { i += 1 }
        return i
    }

    private static func numberEnd(_ chars: [Character], from start: Int) -> Int {
        var i = start
        while i < chars.count, chars[i].isHexDigit || chars[i] == "_" || chars[i] == "x" || (chars[i] == "." && i + 1 < chars.count && chars[i + 1].isNumber) { i += 1 }
        return i
    }
}
