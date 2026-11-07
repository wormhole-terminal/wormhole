// The languages the diff and the viewer colour, by file extension. The GPUI app parses the whole file with
// tree-sitter; this lexes line by line with the state a block comment or a triple-quoted string carries from one
// line to the next, so it is a lexer's approximation (keywords, strings, comments, numbers, types, calls). A file of
// any other kind is left uncoloured, as the app does for text without a grammar. A tree-sitter highlighter
// replaces it through `SyntaxHighlighting`.

public struct SyntaxLanguage: Sendable {
    let keywords: Set<String>
    let constants: Set<String>
    /// What starts a comment that runs to the end of the line.
    let lineComment: String?
    /// The pair that opens and closes a comment across lines (`/*` `*/`).
    let blockComment: (open: String, close: String)?
    let singleQuotes: Bool
    /// Python's `"""` and `'''` strings run over lines.
    let tripleQuotes: Bool
    /// Rust marks macros with `!` and lifetimes with `'`.
    let rust: Bool

    init(
        keywords: Set<String>, constants: Set<String>, lineComment: String? = "//",
        blockComment: (open: String, close: String)? = ("/*", "*/"), singleQuotes: Bool = false,
        tripleQuotes: Bool = false, rust: Bool = false
    ) {
        (self.keywords, self.constants, self.lineComment, self.blockComment) = (keywords, constants, lineComment, blockComment)
        (self.singleQuotes, self.tripleQuotes, self.rust) = (singleQuotes, tripleQuotes, rust)
    }

    public static func of(path: String) -> SyntaxLanguage? {
        guard let name = path.split(separator: "/").last, let dot = name.lastIndex(of: "."), dot != name.startIndex else { return nil }
        switch name[name.index(after: dot)...].lowercased() {
        case "rs": return rust
        case "swift": return swift
        case "ts", "tsx", "js", "jsx", "mjs", "cjs": return script
        case "py": return python
        case "sh", "zsh", "bash", "toml", "yml", "yaml", "rb": return shell
        case "c", "h", "cc", "cpp", "hpp", "m", "java", "kt", "go": return clike
        case "json": return SyntaxLanguage(keywords: [], constants: ["true", "false", "null"], lineComment: nil, blockComment: nil)
        default: return nil
        }
    }

    private static let rust = SyntaxLanguage(
        keywords: ["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where", "while"],
        constants: ["true", "false", "None", "Some", "Ok", "Err"], rust: true)
    private static let swift = SyntaxLanguage(
        keywords: ["actor", "async", "await", "break", "case", "class", "continue", "default", "defer", "do", "else", "enum", "extension", "for", "func", "guard", "if", "import", "in", "init", "let", "private", "public", "internal", "return", "self", "static", "struct", "switch", "throw", "throws", "try", "var", "where", "while"],
        constants: ["true", "false", "nil"])
    private static let script = SyntaxLanguage(
        keywords: ["async", "await", "break", "case", "catch", "class", "const", "continue", "default", "else", "export", "extends", "finally", "for", "from", "function", "if", "import", "interface", "let", "new", "of", "return", "switch", "throw", "try", "type", "var", "while", "yield"],
        constants: ["true", "false", "null", "undefined"], singleQuotes: true)
    private static let python = SyntaxLanguage(
        keywords: ["and", "as", "assert", "async", "await", "class", "def", "elif", "else", "except", "finally", "for", "from", "if", "import", "in", "is", "lambda", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield"],
        constants: ["True", "False", "None"], lineComment: "#", blockComment: nil, singleQuotes: true, tripleQuotes: true)
    private static let shell = SyntaxLanguage(
        keywords: ["case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in", "then", "while", "def", "end", "class", "module"],
        constants: ["true", "false", "nil"], lineComment: "#", blockComment: nil, singleQuotes: true)
    private static let clike = SyntaxLanguage(
        keywords: ["break", "case", "class", "const", "continue", "default", "do", "else", "enum", "for", "func", "fun", "if", "import", "package", "private", "public", "return", "static", "struct", "switch", "typedef", "var", "val", "void", "while", "interface", "extends", "new"],
        constants: ["true", "false", "null", "nil", "NULL"], singleQuotes: true)
}
