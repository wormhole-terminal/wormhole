// The token list's bands: the catalogue's groups in its order (`UI_TOKENS`, then the metrics as "Shape"), then
// the terminal's colours, its sixteen palette slots and the syntax colours that have a rule of their own
// (`studio/draft.rs` `groups`). The catalogue comes from core through the bundled dump.

import DesignSystem

public struct TokenGroup: Sendable, Equatable, Identifiable {
    public let name: String
    public let tokens: [String]
    public var id: String { name }

    static let terminalColors = ["terminal.background", "terminal.foreground", "terminal.cursor", "terminal.selection_background"]
    static let ansi = (0..<16).map { "terminal.ansi.\($0)" }

    /// Every band. `syntaxKeys` are the keys a theme styles; those that follow another (`keyword.control` follows
    /// `keyword`) are not listed, as they have no value of their own.
    public static func all(library: ThemeLibrary = .bundled, syntaxRoots: [String]) -> [TokenGroup] {
        var groups: [TokenGroup] = []
        for info in library.catalog {
            if let last = groups.last, last.name == info.group {
                groups[groups.count - 1] = TokenGroup(name: last.name, tokens: last.tokens + [info.name])
            } else {
                groups.append(TokenGroup(name: info.group, tokens: [info.name]))
            }
        }
        groups.append(TokenGroup(name: "Terminal", tokens: terminalColors))
        groups.append(TokenGroup(name: "Terminal ANSI", tokens: ansi))
        groups.append(TokenGroup(name: "Syntax", tokens: syntaxRoots.map { "syntax.\($0)" }))
        return groups
    }

    public static func group(of token: String, in groups: [TokenGroup]) -> String? {
        groups.first { $0.tokens.contains(token) }?.name
    }
}
