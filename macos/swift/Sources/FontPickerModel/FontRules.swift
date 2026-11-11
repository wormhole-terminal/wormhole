// The picker's rules, which are core's: which families a list shows for a query and in what order
// (`theme::fonts::grouped_font_choices`), what the user's fonts folder holds (`custom::scan`) and what ships in
// the app. `SettingsBridge` answers from the Rust core through the FFI; `LocalFontRules` is the same rules in
// Swift for a build without it and for tests, and a test over the real core keeps the two from drifting.

import Foundation

public protocol FontRules: Sendable {
    /// The family that ships inside the app.
    var bundledFamily: String { get }
    /// The bundled family's font files, to make it available to this process; none when the app has no folder of them
    /// (an installed family of the same name then draws it).
    var bundledFiles: [URL] { get }
    /// Reads the user's fonts folder. Touches the disk, so the library calls it off the main actor.
    func scanCustomFonts() -> CustomFonts
    /// The families to list for `query`, grouped bundled, custom, installed.
    func choices(in catalog: FontCatalog, query: String, monospaceOnly: Bool) -> [FontChoice]
}

/// `grouped_font_choices` in Swift: hidden families (a leading `.`) left out, every word of the query must be in
/// the name, names starting with the query first, then alphabetical, each family once, and the groups in order.
public struct LocalFontRules: FontRules {
    public let bundledFamily = "JetBrains Mono"  // text: a font family's name
    public let bundledFiles: [URL]
    private let custom: CustomFonts

    public init(custom: CustomFonts = CustomFonts(), bundledFiles: [URL] = []) {
        (self.custom, self.bundledFiles) = (custom, bundledFiles)
    }

    public func scanCustomFonts() -> CustomFonts { custom }

    public func choices(in catalog: FontCatalog, query: String, monospaceOnly: Bool) -> [FontChoice] {
        let query = query.trimmingCharacters(in: .whitespaces).lowercased()
        let words = query.split(whereSeparator: \.isWhitespace).map(String.init)
        let all = catalog.installed + catalog.bundled + catalog.customNames
        let ranked = all.compactMap { name -> (notPrefix: Bool, lower: String, name: String)? in
            guard !name.isEmpty, !name.hasPrefix(".") else { return nil }
            if monospaceOnly, let mono = catalog.monospace, !mono.contains(name) { return nil }
            let lower = name.lowercased()
            guard words.allSatisfy(lower.contains) else { return nil }
            return (!query.isEmpty && !lower.hasPrefix(query), lower, name)
        }
        .sorted { a, b in
            if a.notPrefix != b.notPrefix { return !a.notPrefix }
            if a.lower != b.lower { return a.lower.unicodeScalars.lexicographicallyPrecedes(b.lower.unicodeScalars) }
            return a.name.unicodeScalars.lexicographicallyPrecedes(b.name.unicodeScalars)
        }
        var seen = Set<String>()
        let bundled = Set(catalog.bundled), custom = Set(catalog.customNames)
        return ranked.filter { seen.insert($0.name).inserted }
            .map { FontChoice(source: bundled.contains($0.name) ? .bundled : custom.contains($0.name) ? .custom : .installed, name: $0.name) }
            .enumerated().sorted { ($0.element.source, $0.offset) < ($1.element.source, $1.offset) }.map(\.element)
    }
}
