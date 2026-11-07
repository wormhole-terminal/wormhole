// The theme being edited, as the small file it is: only what the person sets, expressions as typed. Everything
// derived stays in Rust; this type knows where a value sits (the top, or one version of a pair) and nothing of
// what it means. Reading and writing are by token name, the same names as everywhere else.

import DesignSystem
import Foundation

public struct ThemeFile: Sendable, Equatable {
    var root: [String: JSONValue]

    public init(name: String? = nil, extends: String? = nil) {
        root = [:]
        self.name = name
        self.extends = extends
    }

    /// Parses a theme file (comments and trailing commas allowed); throws with a reason fit to show.
    public init(parsing text: String) throws { root = try JSONC.parse(text).object ?? [:] }

    /// The file as the Studio writes it.
    public func json() -> String { JSONValue.object(root).pretty() }

    public var name: String? {
        get { root["name"]?.string }
        set { root["name"] = newValue.map(JSONValue.string) }
    }

    public var extends: String? {
        get { root["extends"]?.string }
        set { root["extends"] = newValue.map(JSONValue.string) }
    }

    /// The appearance the file states for the theme (or the shown version), if it does.
    public func appearance(in version: String? = nil) -> ThemeAppearance? {
        let stated = layerFields(at: version.flatMap(index(of:)))["appearance"]?.string ?? root["appearance"]?.string
        return stated.flatMap(ThemeAppearance.init(rawValue:))
    }

    /// The names of the versions of a theme that ships several ("Light", "Dark"); empty for one theme.
    public var versions: [String] { (root["variants"]?.array ?? []).compactMap { $0.object?["name"]?.string } }
    public var isPair: Bool { versions.count > 1 }

    /// What selects `version` in the resolver: "<file name> <version>", or the file's name for one theme.
    public func themeName(version: String?) -> String {
        let family = name ?? ""
        guard let version, versions.contains(version) else { return family }
        return family.isEmpty ? version : "\(family) \(version)"
    }

    private func index(of version: String) -> Int? { versions.firstIndex(of: version) }

    private func layerFields(at index: Int?) -> [String: JSONValue] {
        guard let index else { return [:] }
        return root["variants"]?.array?[index].object ?? [:]
    }

    func layer(at index: Int?) -> ThemeLayer {
        index == nil ? ThemeLayer(fields: root) : ThemeLayer(fields: layerFields(at: index))
    }

    mutating func store(_ layer: ThemeLayer, at index: Int?) {
        guard let index else { root = layer.fields; return }
        var all = root["variants"]?.array ?? []
        all[index] = .object(layer.fields)
        root["variants"] = .array(all)
    }
}
