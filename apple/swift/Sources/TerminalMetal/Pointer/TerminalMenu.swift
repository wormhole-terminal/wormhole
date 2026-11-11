// What a right-click (or Control-click) opens (`terminal_view/context_menu.rs`). The menu itself is the window's to
// build (`TerminalMenu`, its words, keys and enablement come from the keymap and the core), so the view only decides
// whether the click is the menu's, and offers the facts about what is under the pointer. A program that has the mouse
// keeps the click; Shift takes it back.

import AppKit
import DesignSystem
import TerminalCore

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
        pointer.lastPoint = convert(event.locationInWindow, from: nil)
        return contextMenu?()
    }

    /// There is a selection to copy.
    public var canCopy: Bool { interaction?.selectionText()?.isEmpty == false }

    /// Puts the pointer over the middle of viewport cell (`row`, `column`), as a move there would: what the context menu offers
    /// for what is under it (a link, a command block) follows. For the control folder, which has no pointer to move.
    public func placePointer(row: Int, column: Int) {
        let grid = cellGrid
        pointer.lastPoint = CGPoint(x: grid.origin + (CGFloat(column) + 0.5) * grid.cellWidth, y: grid.origin + (CGFloat(row) + 0.5) * grid.cellHeight)
    }

    /// The address of the link under the pointer as of its last move or click.
    public var linkAddressUnderPointer: String? {
        guard let point = pointer.lastPoint, cellGrid.contains(point), let hit = cellGrid.hit(point) else { return nil }
        return interaction?.link(row: hit.row, column: hit.column)?.uri
    }

    /// Open Link, from the menu: the same scheme rules as a Command-click.
    public func openLink(address: String) {
        guard let target = interaction?.openTarget(for: address) else { return }
        linkOpener(target)
    }

    /// Copy Link, from the menu.
    public func copyLink(address: String) {
        Pasteboard.setText(address)
        onCopied?(address)
    }
}
