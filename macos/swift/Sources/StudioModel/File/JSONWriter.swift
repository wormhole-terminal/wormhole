// A theme file as the Studio writes it: two-space indent, the keys a person expects first (`name`, `extends`,
// `appearance`, `terminal`, `ui`, `syntax`, `metrics`, `variants`), the rest in name order, and a whole number
// written 6, not 6.0 (themes.md "Format"). Deterministic, so a save with no change is the same bytes.

import Foundation

extension JSONValue {
    static let keyOrder = ["$schema", "name", "extends", "appearance", "terminal", "ui", "syntax", "metrics", "variants"]

    public func pretty() -> String { write(indent: 0) + "\n" }

    private func write(indent: Int) -> String {
        let pad = String(repeating: "  ", count: indent + 1)
        let close = String(repeating: "  ", count: indent)
        switch self {
        case .null: return "null"
        case .bool(let value): return value ? "true" : "false"
        case .number(let value): return value == value.rounded() && abs(value) < 1e15 ? String(Int(value)) : "\(value)"
        case .string(let value): return Self.quoted(value)
        case .array(let items):
            if items.isEmpty { return "[]" }
            return "[\n" + items.map { pad + $0.write(indent: indent + 1) }.joined(separator: ",\n") + "\n" + close + "]"
        case .object(let fields):
            if fields.isEmpty { return "{}" }
            let keys = fields.keys.sorted { Self.rank($0) == Self.rank($1) ? $0.localizedStandardCompare($1) == .orderedAscending : Self.rank($0) < Self.rank($1) }
            return "{\n" + keys.map { pad + Self.quoted($0) + ": " + fields[$0]!.write(indent: indent + 1) }
                .joined(separator: ",\n") + "\n" + close + "}"
        }
    }

    private static func rank(_ key: String) -> Int { keyOrder.firstIndex(of: key) ?? keyOrder.count }

    private static func quoted(_ text: String) -> String {
        let data = try? JSONSerialization.data(withJSONObject: [text], options: [.withoutEscapingSlashes])
        let array = data.flatMap { String(data: $0, encoding: .utf8) } ?? "[\"\"]"
        return String(array.dropFirst().dropLast())
    }
}
