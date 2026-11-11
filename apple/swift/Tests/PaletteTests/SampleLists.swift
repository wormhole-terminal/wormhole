import PaletteModel

/// A few of the keymap's actions and a small sheet for tests and renders: the real lists are the core's
/// (`PlatformBridgeTests/KeymapListsBridgeTests`).
struct SampleLists: KeymapLists {
    static func install() { ListedKeymap.install(SampleLists()) }

    private static let rows = [
        ActionRow(id: "newTerminal", label: "New terminal", key: Shortcut(.command, "T"), section: .terminal),
        ActionRow(id: "closeTerminal", label: "Close terminal", key: Shortcut(.command, "W"), section: .terminal),
        ActionRow(id: "openFolder", label: "Open folder…", key: Shortcut(.command, "O"), section: .terminal),
        ActionRow(id: "toggleSidebar", label: "Hide sidebar", key: Shortcut([.control, .command], "S"), section: .view),
        ActionRow(id: "showChanges", label: "Show changes", key: Shortcut([.control, .command], "1"), section: .view),
        ActionRow(id: "toggleRightPanel", label: "Hide right panel", key: Shortcut([.option, .command], "I"), section: .view),
        ActionRow(id: "keyboardShortcuts", label: "Keyboard shortcuts", key: Shortcut([.option, .command], "/"), section: .view),
        ActionRow(id: "settings", label: "Settings…", key: Shortcut(.command, ","), section: .app),
    ]

    func actions(_ facts: ActionFacts) -> ActionList {
        ActionList(rows: Self.rows, suggested: ["newTerminal", "openFolder", "showChanges", "toggleRightPanel", "keyboardShortcuts"])
    }

    func sheet(hidden: [String], query: String) -> FilteredShortcuts {
        let all = [
            ShortcutSection(title: "Windows & terminals", rows: [
                ShortcutRow(label: "New terminal", keys: [Shortcut(.command, "T")]),
                ShortcutRow(label: "Terminals 1–9", keys: [Shortcut(.command, "1"), Shortcut(.command, "9")], isRun: true),
            ]),
            ShortcutSection(title: "Find anything", rows: [
                ShortcutRow(label: "Command palette", keys: [Shortcut([.shift, .command], "P")]),
            ]),
        ]
        let found = all.compactMap { section -> ShortcutSection? in
            let rows = section.rows.filter { query.isEmpty || $0.label.localizedCaseInsensitiveContains(query) || $0.keys.contains { $0.glyphs.contains(query) } }
            return rows.isEmpty ? nil : ShortcutSection(title: section.title, rows: rows)
        }
        return FilteredShortcuts(sections: found, shown: found.flatMap(\.rows).count, total: 3, exampleKey: "⌘W")
    }

    func columns(_ heights: [Int], _ count: Int) -> [Range<Int>] { heights.indices.map { $0..<($0 + 1) } }
}
