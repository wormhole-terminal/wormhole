// The bundled themes, read once from `Resources/themes.json` (`apple/scripts/dump-themes.sh` writes it
// from `wormhole-core`). A file this code can't read is a build error, not something to limp past, so
// loading traps with the reason.

import Foundation

public struct TokenInfo: Sendable, Equatable {
    public let name: String
    public let group: String
    public let description: String
}

public struct ThemeLibrary: Sendable {
    public static let formatVersion = 2
    public let themes: [Theme]
    public let defaultDark: String
    public let defaultLight: String
    /// The window opacity and blur the app starts with; the dump's `translucent` colours are for this look.
    public let defaultLook: WindowLook
    /// Every token with its group and what it is for, in the Studio's order (`UI_TOKENS` then `METRIC_TOKENS`).
    public let catalog: [TokenInfo]

    public static let bundled = load(from: Bundle.module.url(forResource: "themes", withExtension: "json"))

    public func theme(id: String) -> Theme? { themes.first { $0.id == id } }

    /// Parses a dump; throws a `DecodingError` for a file that is not the current format.
    public init(json: Data) throws {
        let dump = try JSONDecoder().decode(Dump.self, from: json)
        guard dump.format == Self.formatVersion else {
            throw DecodingError.dataCorrupted(
                .init(codingPath: [], debugDescription: "themes.json format \(dump.format), expected \(Self.formatVersion)"))  // text: a developer diagnostic of a bundled file, never shown in the interface
        }
        themes = dump.themes.map(\.theme)
        defaultDark = dump.defaultDark
        defaultLight = dump.defaultLight
        defaultLook = dump.defaultLook
        catalog = dump.catalog.map { TokenInfo(name: $0[0], group: $0[1], description: $0[2]) }
    }

    private static func load(from url: URL?) -> ThemeLibrary {
        do {
            guard let url else { throw CocoaError(.fileNoSuchFile) }
            return try ThemeLibrary(json: Data(contentsOf: url))
        } catch {
            fatalError("DesignSystem: themes.json is missing or unreadable: \(error)")
        }
    }
}
