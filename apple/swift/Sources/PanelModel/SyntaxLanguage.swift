// The few languages the diff colours, by file extension. The GPUI app parses the whole file with tree-sitter;
// this reads one line at a time, so it is a lexer's approximation (keywords, strings, comments, numbers,
// types, calls), and a file of any other kind is left uncoloured, as the app does for text without a grammar.

public struct SyntaxLanguage: Sendable {
    let keywords: Set<String>
    let constants: Set<String>
    /// What starts a comment that runs to the end of the line.
    let lineComment: String?
    let singleQuotes: Bool
    /// Rust marks macros with `!` and lifetimes with `'`.
    let rust: Bool

    public static func of(path: String) -> SyntaxLanguage? {
        switch path.split(separator: ".").last.map(String.init)?.lowercased() {
        case "rs": rust
        case "swift": swift
        case "ts", "tsx", "js", "jsx", "mjs": script
        case "py": python
        case "sh", "zsh", "bash", "toml", "yml", "yaml": shell
        case "json": SyntaxLanguage(keywords: [], constants: ["true", "false", "null"], lineComment: nil, singleQuotes: false, rust: false)
        default: nil
        }
    }

    private static let rust = SyntaxLanguage(
        keywords: ["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where", "while"],
        constants: ["true", "false", "None", "Some", "Ok", "Err"], lineComment: "//", singleQuotes: false, rust: true)
    private static let swift = SyntaxLanguage(
        keywords: ["actor", "async", "await", "break", "case", "class", "continue", "default", "defer", "do", "else", "enum", "extension", "for", "func", "guard", "if", "import", "in", "init", "let", "private", "public", "internal", "return", "self", "static", "struct", "switch", "throw", "throws", "try", "var", "where", "while"],
        constants: ["true", "false", "nil"], lineComment: "//", singleQuotes: false, rust: false)
    private static let script = SyntaxLanguage(
        keywords: ["async", "await", "break", "case", "catch", "class", "const", "continue", "default", "else", "export", "extends", "finally", "for", "from", "function", "if", "import", "interface", "let", "new", "of", "return", "switch", "throw", "try", "type", "var", "while", "yield"],
        constants: ["true", "false", "null", "undefined"], lineComment: "//", singleQuotes: true, rust: false)
    private static let python = SyntaxLanguage(
        keywords: ["and", "as", "assert", "async", "await", "class", "def", "elif", "else", "except", "finally", "for", "from", "if", "import", "in", "is", "lambda", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield"],
        constants: ["True", "False", "None"], lineComment: "#", singleQuotes: true, rust: false)
    private static let shell = SyntaxLanguage(
        keywords: ["case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in", "then", "while"],
        constants: ["true", "false"], lineComment: "#", singleQuotes: true, rust: false)
}
