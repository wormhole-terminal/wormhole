// A `config.json` value on its way over the seam: Rust holds JSON text, the model holds `JSONValue`.

import Foundation
import SettingsModel

extension JSONValue {
    /// The value as JSON text, for `SettingsHandle.set`.
    func jsonText() -> String {
        // A top-level scalar needs the fragment option; a value that cannot encode (never one built here) is null.
        let data = try? JSONEncoder().encode(self)
        return data.flatMap { String(data: $0, encoding: .utf8) } ?? "null"
    }

    /// A JSON object as `[key: value]`, as `SettingsHandle.values` returns it; empty for text that is not one.
    static func map(from text: String) -> [String: JSONValue] {
        (try? JSONDecoder().decode([String: JSONValue].self, from: Data(text.utf8))) ?? [:]
    }
}
