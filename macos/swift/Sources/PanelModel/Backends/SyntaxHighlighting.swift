// The seam for colouring code. The lexer below is the fallback that needs nothing; tree-sitter, run in Rust through
// the FFI as the GPUI app does, plugs in behind the same call and sees the whole file, not a line.

/// Turns the lines of one file into runs the theme colours.
public protocol SyntaxHighlighting: Sendable {
    /// One array of spans per line, in order, character offsets within the line. Empty where a line is plain or
    /// the file's kind has no grammar.
    func highlight(path: String, lines: [String]) -> [[SyntaxSpan]]
}

/// The lexer as a highlighter: lines in order, the state of each carried into the next.
public struct LexerHighlighter: SyntaxHighlighting {
    public init() {}

    public func highlight(path: String, lines: [String]) -> [[SyntaxSpan]] {
        guard let language = SyntaxLanguage.of(path: path) else { return lines.map { _ in [] } }
        var state = SyntaxState.plain
        return lines.map { SyntaxLexer.spans(in: $0, language: language, state: &state) }
    }
}
