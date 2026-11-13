// The Files tab's path menu as one native menu (the control folder's `files-menu`): the same items as the button's SwiftUI menu
// (`FilesPathButton`), both made from `PathMenuModel`, so a script can list them and pick one.

import AppKit
import DesignSystem
import PanelModel

@MainActor
public enum FilesPathMenu {
    /// Nil while the tree has no folder yet.
    public static func menu(for store: FilesStore) -> NSMenu? {
        guard let model = store.pathMenu else { return nil }
        let menu = NSMenu()
        menu.autoenablesItems = false
        for (index, folder) in model.folders.enumerated() {
            menu.addItem(ActionMenuItem(folder.name, checked: index == 0) { if index > 0 { store.choose(folder.path) } })
        }
        if !model.pinned.isEmpty {
            menu.addItem(.separator())
            for pin in model.pinned { menu.addItem(ActionMenuItem(pin.name) { store.choose(pin.path) }) }
        }
        menu.addItem(.separator())
        let pin = model.isPinned
            ? panelText("files-menu-unpin", ["name": .text(model.shownName)], "Unpin \u{201c}\(model.shownName)\u{201d}")
            : panelText("files-menu-pin", ["name": .text(model.shownName)], "Pin \u{201c}\(model.shownName)\u{201d}")
        menu.addItem(ActionMenuItem(pin) { store.setPinned("", !model.isPinned) })
        menu.addItem(ActionMenuItem(panelText("files-menu-choose-folder", "Choose Folder\u{2026}")) { store.chooseFolder() })
        if model.canFollow { menu.addItem(ActionMenuItem(panelText("files-menu-follow-terminal", "Follow Terminal")) { store.followTerminal() }) }
        return menu
    }
}
