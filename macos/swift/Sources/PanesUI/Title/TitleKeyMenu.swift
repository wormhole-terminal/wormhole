// The title strip's menu for the keyboard (⌃↩ with the keys on the strip, `window/focus/menu.rs`): the folder's path
// menu, or the row's own where the row shows no folder. A SwiftUI `.contextMenu` cannot be opened from code, so the
// keys get the same entries (`TitleMenuEntries`) as a native menu; the control folder's `title-menu` and `path-menu` read
// these, which hold what a right-click shows.

import AppKit
import DesignSystem
import PanesModel

@MainActor
public enum TitleKeyMenu {
    public static func menu(facts: TitleFacts, actions: TitleActions) -> NSMenu {
        if let folder = facts.folder, facts.row.shows(.folder) {
            return native(TitleMenuEntries.path(folder, shown: facts.shownFolder ?? folder, actions))
        }
        return rowMenu(facts.row, actions: actions)
    }

    /// The row's own menu, as a right-click on the row (not on its folder) shows it (the control folder's `title-menu`).
    public static func rowMenu(_ row: TitleRow, actions: TitleActions) -> NSMenu {
        native(TitleMenuEntries.row(row, actions))
    }

    /// The folder's own menu, as a right-click on the folder shows it (the control folder's `path-menu`), from the row's facts.
    public static func pathMenu(facts: TitleFacts, actions: TitleActions) -> NSMenu? {
        guard let shown = facts.shownFolder ?? facts.folder else { return nil }
        return native(TitleMenuEntries.path(facts.folder, shown: shown, actions))
    }

    private static func native(_ entries: [TitleMenuEntry]) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        for entry in entries {
            switch entry {
            case .heading(let text): menu.addItem(.sectionHeader(title: text))
            case .divider: menu.addItem(.separator())
            case .item(let title, let enabled, let checked, let icon, let run):
                let item = ActionMenuItem(title, enabled: enabled, checked: checked ?? false, run: run)
                if let icon { item.image = TitleMenuEntries.icon(of: icon) }
                menu.addItem(item)
            }
        }
        return menu
    }
}
