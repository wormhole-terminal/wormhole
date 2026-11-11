// A theme's terminal colours: what the grid draws with, and what the UI's own surfaces derive from.

public struct TerminalPalette: Sendable, Equatable {
    public let foreground: RGBA
    public let background: RGBA
    public let cursor: RGBA
    /// Unset in a theme: the terminal draws the glyph under the cursor in its own colour.
    public let cursorText: RGBA?
    public let selectionBackground: RGBA
    public let selectionForeground: RGBA?
    /// The 16 ANSI colours, 0...7 normal and 8...15 bright.
    public let ansi: [RGBA]
}

extension TerminalPalette: Decodable {}

/// One syntax key's look (`syntax.keyword`): the diff and the file viewer colour code with these.
public struct SyntaxStyle: Sendable, Equatable {
    public let color: RGBA
    public let bold: Bool
    public let italic: Bool
}

extension SyntaxStyle: Decodable {
    public init(from decoder: any Decoder) throws {
        var parts = try decoder.unkeyedContainer()
        self.init(
            color: try parts.decode(RGBA.self), bold: try parts.decode(Bool.self),
            italic: try parts.decode(Bool.self))
    }
}
