// The menu a right-click (or Control-click) opens: the link under the pointer, Copy, Paste, Select All, Find,
// the splits and Clear, in the app's order (`terminal_view/context_menu.rs`). Items send their action up the
// responder chain, so the window answers the ones that are its own (find, splits) and the menu shows only
// what the chain would take. A program that has the mouse keeps the click; Shift takes it back.

import AppKit
import TerminalCore

/// Actions the window answers, sent up the responder chain from the context menu. The names are the
/// selectors a window controller implements.
public enum TerminalWindowAction {
    public static let find = Selector(("showTerminalFind:"))
    public static let splitRight = Selector(("splitTerminalRight:"))
    public static let splitDown = Selector(("splitTerminalDown:"))
}

extension MetalTerminalView {
    /// Whether a press of this kind opens the menu instead of reaching the program or the selection.
    static func opensMenu(program takesPointer: Bool, shift: Bool) -> Bool { !takesPointer || shift }

    func openContextMenu(for event: NSEvent) -> Bool {
        guard let menu = menu(for: event) else { return false }
        pointer.lastPoint = convert(event.locationInWindow, from: nil)
        NSMenu.popUpContextMenu(menu, with: event, for: self)
        return true
    }

    public override func menu(for event: NSEvent) -> NSMenu? {
        guard Self.opensMenu(program: interaction?.takesPointer ?? false, shift: event.modifierFlags.contains(.shift)) else { return nil }
        let menu = NSMenu()
        menu.autoenablesItems = true
        // Items for this view name it; the window's go up the responder chain (no target).
        let own = { (word: TerminalWord, symbol: String, action: Selector, uri: String?) in
            self.item(word, symbol, action, target: self, represented: uri)
        }
        let window = { (word: TerminalWord, symbol: String, action: Selector) in self.item(word, symbol, action, target: nil, represented: nil) }
        let point = convert(event.locationInWindow, from: nil)
        if let hit = cellGrid.hit(point), cellGrid.contains(point), let link = interaction?.link(row: hit.row, column: hit.column) {
            menu.addItem(own(.menuOpenLink, "safari", #selector(openHoveredLink(_:)), link.uri))
            menu.addItem(own(.menuCopyLink, "link", #selector(copyHoveredLink(_:)), link.uri))
            menu.addItem(.separator())
        }
        menu.addItem(own(.menuCopy, "doc.on.doc", #selector(copy(_:)), nil))
        menu.addItem(own(.menuPaste, "doc.on.clipboard", #selector(paste(_:)), nil))
        menu.addItem(own(.menuSelectAll, "selection.pin.in.out", #selector(selectAll(_:)), nil))
        menu.addItem(.separator())
        menu.addItem(window(.menuFind, "magnifyingglass", TerminalWindowAction.find))
        menu.addItem(.separator())
        menu.addItem(window(.menuSplitRight, "rectangle.split.2x1", TerminalWindowAction.splitRight))
        menu.addItem(window(.menuSplitDown, "rectangle.split.1x2", TerminalWindowAction.splitDown))
        menu.addItem(.separator())
        menu.addItem(own(.menuClear, "eraser", #selector(clearBuffer(_:)), nil))
        return menu
    }

    private func item(_ word: TerminalWord, _ symbol: String, _ action: Selector, target: AnyObject?, represented: String?) -> NSMenuItem {
        let item = NSMenuItem(title: word.text, action: action, keyEquivalent: "")
        item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        item.target = target
        item.representedObject = represented
        return item
    }

    @objc func openHoveredLink(_ sender: NSMenuItem) {
        guard let uri = sender.representedObject as? String, let target = interaction?.openTarget(for: uri) else { return }
        linkOpener(target)
    }

    @objc func copyHoveredLink(_ sender: NSMenuItem) {
        guard let uri = sender.representedObject as? String else { return }
        let board = NSPasteboard.general
        board.clearContents()
        board.setString(uri, forType: .string)
        onCopied?(uri)
    }
}
