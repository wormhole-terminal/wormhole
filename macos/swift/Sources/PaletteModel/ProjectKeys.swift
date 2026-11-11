// A trusted project's own keys under the sheet's sections (`shortcut_sheet.rs` `ProjectKeys`): the rows its actions' keys
// make, as the filter leaves them. The rule that finds a row is the core's (`ListedKeymap.matches`).

import Foundation

public struct ProjectKeys: Sendable, Equatable {
    /// "terminal/.wormhole/actions.json".
    public var name: String
    /// The rows the filter leaves, and how many the project has in all (the footer counts them).
    public var rows: [ShortcutRow]
    public var total: Int

    /// `nil` for a project with no action that has a key.
    public init?(_ project: PaletteProject?, query: String) {
        guard let project else { return nil }
        let keyed = project.actions.compactMap { action in action.key.flatMap(Shortcut.init(glyphs:)).map { (action.title, action.key ?? "", $0) } }
        guard !keyed.isEmpty else { return nil }
        name = project.name + "/actions.json"
        total = keyed.count
        rows = keyed.compactMap { title, glyphs, key in
            ListedKeymap.matches(query, label: title, key: glyphs).map { ShortcutRow(label: title, keys: [key], hit: $0.hit) }
        }
    }
}

extension FilteredShortcuts {
    /// The sheet with the project's block counted in.
    public func counting(_ project: ProjectKeys?) -> FilteredShortcuts {
        var counted = self
        counted.shown += project?.rows.count ?? 0
        counted.total += project?.total ?? 0
        return counted
    }
}
