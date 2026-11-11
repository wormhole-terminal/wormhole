@testable import DesignSystem
import Foundation
import Synchronization

/// Helpers for the theme tests: colours from hex, and one dump entry rewritten as a resolver's answer.
enum ThemeStoreFixtures {
    static func rgba(hex: String) -> RGBA? {
        let digits = hex.dropFirst()
        guard digits.count == 6, let value = Int(digits, radix: 16) else { return nil }
        return RGBA(Double(value >> 16 & 255) / 255, Double(value >> 8 & 255) / 255, Double(value & 255) / 255)
    }

    /// The dump's entry for `id` in the resolver's shape (`normal` only), as `resolve_theme` answers.
    static func resolved(id: String) throws -> Data {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../Sources/DesignSystem/Resources/themes.json").standardized
        let dump = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any]
        let themes = dump?["themes"] as? [[String: Any]] ?? []
        guard var entry = themes.first(where: { $0["id"] as? String == id }) else { throw CocoaError(.fileReadUnknown) }
        entry["highContrast"] = nil
        return try JSONSerialization.data(withJSONObject: entry)
    }

    final class Log: Sendable {
        private let requests = Mutex<[ThemeRequest]>([])
        func add(_ request: ThemeRequest) { requests.withLock { $0.append(request) } }
        var last: ThemeRequest? { requests.withLock { $0.last } }
    }
}
