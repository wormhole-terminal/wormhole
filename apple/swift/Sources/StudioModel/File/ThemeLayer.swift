// One layer of a theme file: the top (shared by every version) or one entry of `variants`. Reads and writes a
// token's own value as typed (`#7aa2f7`, `primary / 15%`, `6`), leaving every other key as it was.

import Foundation

struct ThemeLayer {
    var fields: [String: JSONValue]

    func value(of path: TokenPath) -> String? {
        let (section, key) = path.location
        let object = section == "terminal.ansi" ? fields["terminal"]?.object?["ansi"]?.object : fields[section]?.object
        switch object?[key] {
        case .string(let text)?: return text
        case .number(let number)?: return JSONValue.number(number).pretty().trimmingCharacters(in: .newlines)
        case .object(let style)?: return style["color"]?.string  // a syntax key written as a style
        default: return nil
        }
    }

    /// Sets the value, or removes it with `nil`; an emptied section goes with it.
    mutating func set(_ path: TokenPath, _ value: String?) {
        let (section, key) = path.location
        if section == "terminal.ansi" {
            update("terminal") { terminal in
                var ansi = terminal["ansi"]?.object ?? [:]
                ansi[key] = value.map(JSONValue.string)
                terminal["ansi"] = ansi.isEmpty ? nil : .object(ansi)
            }
        } else {
            update(section) { $0[key] = Self.written(value, for: path, old: $0[key]) }
        }
    }

    /// A syntax key keeps the bold and italic it may have next to its colour.
    private static func written(_ value: String?, for path: TokenPath, old: JSONValue?) -> JSONValue? {
        if case .object(var style)? = old, case .syntax = path {
            style["color"] = value.map(JSONValue.string)
            return style.isEmpty ? nil : .object(style)
        }
        guard let value else { return nil }
        if case .metric = path {
            return Double(value.replacingOccurrences(of: "px", with: "").trimmingCharacters(in: .whitespaces)).map(JSONValue.number)
        }
        return .string(value)
    }

    private mutating func update(_ section: String, _ change: (inout [String: JSONValue]) -> Void) {
        var object = fields[section]?.object ?? [:]
        change(&object)
        fields[section] = object.isEmpty ? nil : .object(object)
    }
}
