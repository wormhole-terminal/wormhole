import Foundation

/// A read-out as the app writes it (`Sources/DesignSystem/Driving/Readout.swift`, `Readout.line`), a line each:
///
///     kind word… "text"… key="value"… flag…
///
/// Bare words (ids, numbers, flags) in order, quoted texts in order, `key="value"` pairs. A quoted text never holds a
/// straight quote or a line break (the app writes `”` and `⏎`), so a quote always opens or closes one.
struct Readout {
    struct Line {
        var kind: String
        /// The bare words after the kind, flags included (`changed`, `off`, `picked`).
        var words: [String] = []
        var quoted: [String] = []
        var pairs: [String: String] = [:]
        /// Indented under the line before it (a row's caption, a shelf's card).
        var indented = false
        let text: String

        func has(_ flag: String) -> Bool { words.contains(flag) }
        subscript(_ key: String) -> String? { pairs[key] }
        /// The first quoted text, the line's label.
        var label: String { quoted.first ?? "" }
    }

    let lines: [Line]
    let text: String

    init(_ text: String) {
        self.text = text
        lines = text.split(separator: "\n").map { Self.parse(String($0)) }
    }

    func all(_ kind: String) -> [Line] { lines.filter { $0.kind == kind } }
    func first(_ kind: String) -> Line? { lines.first { $0.kind == kind } }
    /// The first line of `kind` whose first word is `id` (`row opacity`, `card 0`, `tab 4`).
    func line(_ kind: String, _ id: String) -> Line? { lines.first { $0.kind == kind && $0.words.first == id } }

    /// The lines indented under the first line of `kind` with first word `id`, up to the next line that is not indented.
    func under(_ kind: String, _ id: String) -> [Line] {
        guard let start = lines.firstIndex(where: { $0.kind == kind && $0.words.first == id }) else { return [] }
        return Array(lines[(start + 1)...].prefix { $0.indented })
    }

    static func parse(_ line: String) -> Line {
        var tokens: [String] = []
        var current = ""
        var inQuote = false
        for character in line {
            if character == "\"" { inQuote.toggle() }
            if character == " " && !inQuote {
                if !current.isEmpty { tokens.append(current) }
                current = ""
            } else {
                current.append(character)
            }
        }
        if !current.isEmpty { tokens.append(current) }
        var parsed = Line(kind: tokens.first ?? "", indented: line.hasPrefix("  "), text: line)
        for token in tokens.dropFirst() {
            if token.hasPrefix("\""), token.hasSuffix("\""), token.count >= 2 {
                parsed.quoted.append(String(token.dropFirst().dropLast()))
            } else if let equals = token.firstIndex(of: "="), token[token.index(after: equals)...].hasPrefix("\"") {
                let value = token[token.index(after: equals)...]
                parsed.pairs[String(token[..<equals])] = String(value.dropFirst().dropLast())
            } else {
                parsed.words.append(token)
            }
        }
        return parsed
    }
}
