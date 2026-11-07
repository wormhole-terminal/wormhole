// Cuts one line of code into runs, each named by the syntax key the theme colours (`keyword`, `string`, ...;
// `wormhole_core::theme::tokens` `syntax.*`). Characters between runs are plain and take the text colour.

public struct SyntaxSpan: Equatable, Sendable {
    public var range: Range<Int>
    public var key: String
}

public enum SyntaxLexer {
    public static func spans(in line: String, language: SyntaxLanguage) -> [SyntaxSpan] {
        let chars = Array(line)
        var spans: [SyntaxSpan] = []
        var i = 0
        func add(_ end: Int, _ key: String) {
            spans.append(SyntaxSpan(range: i..<end, key: key))
            i = end
        }
        while i < chars.count {
            let c = chars[i]
            if let comment = language.lineComment, starts(with: comment, at: i, in: chars) {
                add(chars.count, "comment")
            } else if c == "\"" || (c == "'" && language.singleQuotes) {
                add(end(ofStringAt: i, in: chars), "string")
            } else if c == "'", language.rust {
                // A char literal is closed within a few characters; otherwise it is a lifetime.
                let closes = (i + 2 < chars.count && chars[i + 2] == "'") || (i + 3 < chars.count && chars[i + 1] == "\\" && chars[i + 3] == "'")
                add(closes ? end(ofStringAt: i, in: chars) : identifierEnd(chars, from: i + 1), closes ? "string" : "attribute")
            } else if c.isNumber {
                add(numberEnd(chars, from: i), "number")
            } else if c.isLetter || c == "_" {
                let end = identifierEnd(chars, from: i)
                let word = String(chars[i..<end])
                let next = end < chars.count ? chars[end] : " "
                let afterDot = i > 0 && chars[i - 1] == "."
                if language.keywords.contains(word) { add(end, "keyword") }
                else if language.constants.contains(word) { add(end, "boolean") }
                else if language.rust, next == "!" { add(end + 1, "function.macro") }
                else if next == "(" { add(end, afterDot ? "function.method" : "function") }
                else if afterDot { add(end, "property") }
                else if c.isUppercase { add(end, "type") }
                else { i = end }
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

    private static func starts(with prefix: String, at index: Int, in chars: [Character]) -> Bool {
        let wanted = Array(prefix)
        return index + wanted.count <= chars.count && Array(chars[index..<index + wanted.count]) == wanted
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
