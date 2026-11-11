// The file icon theme in force, answering `FileIconProviding` (`wormhole_core::theme::icon_theme`, whose lookup the FFI
// will bring). Lookup follows Zed's `FileIcons::get_icon`: the full file name and then every suffix after a `.`, longest
// first, is tried against the stems, then the suffixes; the first candidate whose icon type exists wins, else the
// `default` type. Each candidate is tried exactly first and then case-insensitively, so `photo.PNG` gets the image icon.

import Foundation
import PanelModel

public final class BundledIconTheme: FileIconProviding, @unchecked Sendable {
    private struct Variant {
        let root: URL
        let directory: IconThemeFile.Pair
        let named: [String: IconThemeFile.Pair]
        let lowerNamed: [String: IconThemeFile.Pair]
        let stems: [String: String]
        let suffixes: [String: String]
        let lowerStems: [String: String]
        let lowerSuffixes: [String: String]
        let icons: [String: String]
    }

    private let dark: Variant
    private let light: Variant
    public let rendersInColour: Bool

    /// The theme bundled with the app, or nil when its resources are missing.
    public convenience init?() { self.init(bundle: .module) }

    init?(bundle: Bundle) {
        guard let root = bundle.url(forResource: "wormhole-icons", withExtension: "json"),
            let data = try? Data(contentsOf: root), let file = try? JSONDecoder().decode(IconThemeFile.self, from: data)
        else { return nil }
        let folder = root.deletingLastPathComponent()
        func variant(_ appearance: String) -> Variant? {
            guard let theme = file.themes.first(where: { $0.appearance == appearance }) ?? file.themes.first else { return nil }
            let lower = { (map: [String: String]?) in Dictionary((map ?? [:]).map { ($0.key.lowercased(), $0.value) }, uniquingKeysWith: { first, _ in first }) }
            return Variant(
                root: folder, directory: theme.directoryIcons ?? .init(), named: theme.namedDirectoryIcons ?? [:],
                lowerNamed: Dictionary((theme.namedDirectoryIcons ?? [:]).map { ($0.key.lowercased(), $0.value) }, uniquingKeysWith: { first, _ in first }),
                stems: theme.fileStems ?? [:], suffixes: theme.fileSuffixes ?? [:], lowerStems: lower(theme.fileStems), lowerSuffixes: lower(theme.fileSuffixes),
                icons: (theme.fileIcons ?? [:]).mapValues(\.path))
        }
        guard let dark = variant("dark"), let light = variant("light") else { return nil }
        (self.dark, self.light) = (dark, light)
        rendersInColour = file.wormhole?.render != "mono"
    }

    public func file(_ path: String, dark isDark: Bool) -> URL? {
        let theme = isDark ? dark : light
        let name = path.split(separator: "/").last.map(String.init) ?? path
        for candidate in Self.candidates(name) {
            let key = theme.stems[candidate] ?? theme.suffixes[candidate]
                ?? theme.lowerStems[candidate.lowercased()] ?? theme.lowerSuffixes[candidate.lowercased()]
            if let key, let icon = theme.icons[key] { return theme.root.appendingPathComponent(icon) }
        }
        return theme.icons["default"].map { theme.root.appendingPathComponent($0) }
    }

    public func folder(_ name: String, open: Bool, dark isDark: Bool) -> URL? {
        let theme = isDark ? dark : light
        let name = name.split(separator: "/").last.map(String.init) ?? name
        let pair = theme.named[name] ?? theme.lowerNamed[name.lowercased()]
        let path = (open ? pair?.expanded : pair?.collapsed) ?? (open ? theme.directory.expanded : theme.directory.collapsed)
        return path.map { theme.root.appendingPathComponent($0) }
    }

    /// The full name, then everything after each `.`, longest first: `foo.d.ts` is `foo.d.ts`, `d.ts`, `ts`.
    static func candidates(_ name: String) -> [String] {
        var out = [name]
        var index = name.startIndex
        while let dot = name[index...].firstIndex(of: ".") {
            let next = name.index(after: dot)
            if next < name.endIndex { out.append(String(name[next...])) }
            index = next
        }
        return out
    }
}
