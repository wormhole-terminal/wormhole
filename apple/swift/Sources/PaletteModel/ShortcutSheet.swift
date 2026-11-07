// The keyboard-shortcut sheet as data: sections of labelled keys, filterable by label or by the keys
// ("⌘W" finds Close terminal), as core's `keys/keymap/sheet.rs` does. Labels are the sheet's own sentence case.

import Foundation

public struct ShortcutRow: Sendable, Equatable, Identifiable {
    public var label: String
    public var keys: [Shortcut]
    /// A run reads "⌘1 … ⌘9": the first and the last.
    public var isRange: Bool = false
    public var id: String { label }

    public var keysText: String { keys.map(\.glyphs).joined(separator: isRange ? " … " : " ") }
}

public struct ShortcutSection: Sendable, Equatable, Identifiable {
    public var title: String
    public var rows: [ShortcutRow]
    public var id: String { title }
}

public enum ShortcutSheet {
    public static let sections: [ShortcutSection] = [
        ShortcutSection(
            title: CommandCategory.terminals.title,
            rows: [
                ShortcutRow(label: "New terminal", keys: [Shortcuts.newTerminal]),
                ShortcutRow(label: "Close terminal", keys: [Shortcuts.closeTerminal]),
                ShortcutRow(
                    label: "Previous / next terminal", keys: [Shortcuts.previousTerminal, Shortcuts.nextTerminal]),
                ShortcutRow(
                    label: "Terminals 1–9", keys: [Shortcuts.terminal(number: 1), Shortcuts.terminal(number: 9)],
                    isRange: true),
            ]),
        ShortcutSection(
            title: CommandCategory.view.title,
            rows: [
                ShortcutRow(label: "Hide / show sidebar", keys: [Shortcuts.toggleSidebar]),
                ShortcutRow(label: "Hide / show right panel", keys: [Shortcuts.togglePanel]),
                ShortcutRow(
                    label: "Changes · History · Notes",
                    keys: [Shortcuts.panelTab(.changes), Shortcuts.panelTab(.notes)], isRange: true),
            ]),
        ShortcutSection(
            title: CommandCategory.findAnything.title,
            rows: [
                ShortcutRow(label: "Command palette", keys: [Shortcuts.commandPalette]),
                ShortcutRow(label: "Keyboard shortcuts (this sheet)", keys: [Shortcuts.shortcutSheet]),
            ]),
    ]

    /// The sections with only the rows `filter` finds; a section with none is dropped.
    public static func sections(matching filter: String) -> [ShortcutSection] {
        guard !filter.allSatisfy(\.isWhitespace) else { return sections }
        return sections.compactMap { section in
            let rows = section.rows.filter {
                PaletteSearch.score(filter, title: $0.label, other: [$0.keysText]) != nil
            }
            return rows.isEmpty ? nil : ShortcutSection(title: section.title, rows: rows)
        }
    }
}
