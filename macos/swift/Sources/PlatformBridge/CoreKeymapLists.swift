// The palette's Actions and the shortcut sheet from the core's keymap (`keymap::palette`, `keymap::sheet`): which actions
// a window offers and in what words, the sheet's rows, its filter and its columns. Core marks bytes; the views mark
// characters.

import PaletteModel
import WormholeBindings

struct CoreKeymapLists: KeymapLists {
    func actions(_ facts: ActionFacts) -> ActionList {
        let found = keymapPaletteActions(
            state: MenuStateRecord(
                off: facts.off, checked: [], disabled: facts.disabled, hidden: facts.hidden, terminals: [], currentTerminal: nil,
                recent: [], projectActions: [], shells: [], profiles: [], currentProfile: nil, windowProfile: nil,
                split: facts.isSplit, undo: nil, redo: nil, debug: facts.isDebug, persistent: facts.isPersistent))
        let rows = found.actions.compactMap { action -> ActionRow? in
            guard let section = ActionSection(title: action.section) else { return nil }
            return ActionRow(id: action.id, label: action.label, key: action.key.flatMap(Self.shortcut), section: section)
        }
        return ActionList(rows: rows, suggested: found.suggested)
    }

    func sheet(hidden: [String], query: String) -> FilteredShortcuts {
        let found = keymapSheet(hidden: hidden, query: query)
        let sections = found.sections.map { section in
            ShortcutSection(
                title: section.title,
                rows: section.rows.map { row in
                    ShortcutRow(
                        label: row.label, keys: row.keys.compactMap(Self.shortcut), isRun: row.range,
                        hit: row.labelHit.map { characters($0, in: row.label) })
                })
        }
        return FilteredShortcuts(sections: sections, shown: Int(found.shown), total: Int(found.total), exampleKey: found.exampleKey)
    }

    func columns(_ heights: [Int], _ count: Int) -> [Range<Int>] {
        keymapSheetColumns(heights: heights.map(UInt32.init), columns: UInt32(count)).map { Int($0.start)..<Int($0.end) }
    }

    func matches(_ query: String, label: String, key: String) -> ShortcutMatch? {
        keymapSheetMatches(query: query, label: label, key: key).map { ShortcutMatch(hit: $0.labelHit.map { characters($0, in: label) }) }
    }

    private static func shortcut(_ key: KeySpecRecord) -> Shortcut? { Shortcut(glyphs: keySpecGlyphs(key: key)) }
}
