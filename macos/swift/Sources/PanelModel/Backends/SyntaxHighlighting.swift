// The seam for colouring code. The lexer below needs nothing and colours the sample (previews, tests, an unlinked build);
// the app's is tree-sitter, run in Rust through the FFI (`FilesBridge.CoreSyntaxHighlighter`), which is given a whole file's
// lines by the viewer. A real diff's lines arrive coloured (`DiffFile.coloredByCore`).

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
