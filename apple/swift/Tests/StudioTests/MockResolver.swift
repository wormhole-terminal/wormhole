// A stand-in for Rust's resolver, for tests: reads the draft file the workspace wrote, takes the bundled theme it
// extends from `themes.json` and lays the file's hex colours, token aliases and `token / N%` alphas over it. It
// knows nothing of derivation (a changed primary does not move the focus ring): that is core's, covered by the
// FFI-backed tests when the library is linked. What it does faithfully is refuse what core refuses to take.

@testable import DesignSystem
import Foundation

struct MockResolver: ThemeResolving {
    struct Refusal: Error, LocalizedError { let errorDescription: String? }

    func userThemes(in directory: URL) -> Data { Data("[]".utf8) }

    func resolve(_ request: ThemeRequest) throws -> Data {
        guard let directory = request.themesDirectory else { throw Refusal(errorDescription: "no folder") }
        let names = try FileManager.default.contentsOfDirectory(atPath: directory.path)
        var files: [[String: Any]] = []
        for name in names {
            let data = try Data(contentsOf: directory.appendingPathComponent(name))
            if let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] { files.append(object) }
        }
        guard let file = files.first(where: { Self.themeNames($0).contains(request.theme) }) else {
            throw Refusal(errorDescription: "unknown theme \(request.theme)")
        }
        let variant = (file["variants"] as? [[String: Any]])?.first { "\(file["name"] ?? "") \($0["name"] ?? "")" == request.theme }
        var entry = try Self.entry(named: variant?["extends"] as? String ?? file["extends"] as? String ?? "Tokyo Night")
        entry["name"] = request.theme
        if request.increaseContrast { entry["normal"] = entry["highContrast"] }
        for layer in [file, variant ?? [:]] { try Self.apply(layer, to: &entry) }
        if let appearance = variant?["appearance"] ?? file["appearance"] { entry["appearance"] = appearance }
        entry["highContrast"] = nil
        return try JSONSerialization.data(withJSONObject: entry)
    }

    private static func themeNames(_ file: [String: Any]) -> [String] {
        let name = file["name"] as? String ?? ""
        return [name] + ((file["variants"] as? [[String: Any]]) ?? []).map { "\(name) \($0["name"] ?? "")" }
    }

    private static func apply(_ layer: [String: Any], to entry: inout [String: Any]) throws {
        var normal = entry["normal"] as? [String: Any] ?? [:]
        var ui = normal["ui"] as? [String: Any] ?? [:]
        var terminal = entry["terminal"] as? [String: Any] ?? [:]
        for (key, value) in layer["terminal"] as? [String: Any] ?? [:] where key != "ansi" {
            terminal[Self.camel(key)] = try color(value, in: ui)
        }
        var ansi = terminal["ansi"] as? [Any] ?? []
        let named = (layer["terminal"] as? [String: Any])?["ansi"] as? [String: Any] ?? [:]
        for (slot, value) in named {
            if let index = Int(slot), ansi.indices.contains(index) { ansi[index] = try color(value, in: ui) }
        }
        terminal["ansi"] = ansi
        for (key, value) in layer["ui"] as? [String: Any] ?? [:] { ui[key] = try color(value, in: ui) }
        var metrics = normal["metrics"] as? [String: Any] ?? [:]
        for (key, value) in layer["metrics"] as? [String: Any] ?? [:] { metrics[key] = value }
        normal["ui"] = ui
        normal["metrics"] = metrics
        entry["terminal"] = terminal
        entry["normal"] = normal
    }

    private static func color(_ value: Any, in ui: [String: Any]) throws -> [Double] {
        guard let text = (value as? String)?.trimmingCharacters(in: .whitespaces) else { throw Refusal(errorDescription: "not a colour") }
        if text.hasPrefix("#"), text.count == 7, let value = Int(text.dropFirst(), radix: 16) {
            return [Double(value >> 16 & 255) / 255, Double(value >> 8 & 255) / 255, Double(value & 255) / 255, 1]
        }
        let parts = text.components(separatedBy: " / ")
        if parts.count == 2, let base = ui[parts[0]] as? [Double], let percent = Double(parts[1].replacingOccurrences(of: "%", with: "")) {
            return [base[0], base[1], base[2], percent / 100]
        }
        if let alias = ui[text] as? [Double] { return alias }
        throw Refusal(errorDescription: "Unknown token “\(text)” at 1:1.")
    }

    private static func camel(_ key: String) -> String {
        let parts = key.split(separator: "_")
        return parts.first.map(String.init)! + parts.dropFirst().map { $0.capitalized }.joined()
    }

    private static func entry(named name: String) throws -> [String: Any] {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../Sources/DesignSystem/Resources/themes.json").standardized
        let dump = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any]
        guard let entry = (dump?["themes"] as? [[String: Any]])?.first(where: { ($0["name"] as? String)?.lowercased() == name.lowercased() })
        else { throw Refusal(errorDescription: "unknown theme \(name)") }
        return entry
    }
}
