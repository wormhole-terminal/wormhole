// The title strip's menu for the keyboard (⌃↩ with the keys on the strip, `window/focus/menu.rs`): the folder's path
// menu, or the row's own where the row shows no folder. A SwiftUI `.contextMenu` cannot be opened from code, so the
// keys get the same items as a native menu; choosing one runs what the SwiftUI menu's item does.

import AppKit
import DesignSystem
import PanesModel

@MainActor
public enum TitleKeyMenu {
    public static func menu(facts: TitleFacts, actions: TitleActions) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        if let folder = facts.folder, facts.row.shows(.folder) {
            path(folder, shown: facts.shownFolder ?? folder, actions, into: menu)
        } else {
            row(facts.row, actions, into: menu)
        }
        return menu
    }

    /// The row's own menu, as a right-click on the row (not on its folder) opens it (the control folder's `title-menu`).
    public static func rowMenu(_ row: TitleRow, actions: TitleActions) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        self.row(row, actions, into: menu)
        return menu
    }

    /// The folder's own menu, as a right-click on the folder opens it (the control folder's `path-menu`): `shown` is the folder as the
    /// row writes it, `folder` its absolute path.
    public static func pathMenu(folder: String, shown: String, actions: TitleActions) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        path(folder, shown: shown, actions, into: menu)
        return menu
    }

    private static func path(_ folder: String, shown: String, _ actions: TitleActions, into menu: NSMenu) {
        menu.addItem(.sectionHeader(title: shown))
        menu.addItem(.separator())
        menu.addItem(ActionMenuItem(PanesText.openInFinder, run: actions.revealFolder))
        menu.addItem(ActionMenuItem(PanesText.copyPath, run: actions.copyPath))
        menu.addItem(ActionMenuItem(PanesText.insertIntoTerminal, enabled: actions.canInsert, run: actions.insertPath))
        menu.addItem(.separator())
        let volume = (try? URL(fileURLWithPath: "/").resourceValues(forKeys: [.volumeNameKey]).volumeName) ?? "Macintosh HD" // text: macOS's default name of the startup volume
        for step in PathChain.of(folder, bootVolume: volume) {
            let item = ActionMenuItem(step.name) { NSWorkspace.shared.open(URL(fileURLWithPath: step.path)) }
            let icon = NSWorkspace.shared.icon(forFile: step.path)
            icon.size = NSSize(width: 16, height: 16)
            item.image = icon
            menu.addItem(item)
        }
    }

    private static func row(_ shown: TitleRow, _ actions: TitleActions, into menu: NSMenu) {
        let row = actions.ownRow() ?? shown
        menu.addItem(.sectionHeader(title: PanesText.showInTitleRow))
        for part in TitlePart.allCases where part.hideable {
            let shown = row.shows(part)
            menu.addItem(ActionMenuItem(PanesText.partName(part), enabled: actions.canEditRow, checked: shown) { actions.setPart(part, !shown) })
        }
        menu.addItem(.separator())
        menu.addItem(ActionMenuItem(PanesText.resetTitleRow, enabled: actions.canEditRow && !row.isDefault, run: actions.resetRow))
        menu.addItem(ActionMenuItem(PanesText.customizeTitleRow, enabled: actions.canEditRow, run: actions.customizeRow))
    }
}
