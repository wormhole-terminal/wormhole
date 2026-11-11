// A group's context menu (`sidebar/group_popover.rs` `group_menu`): Rename…, the colour dots and Other colour…,
// the folder's items, then the actions. Choosing an item answers with a `ContextAction` of kind `group.<name>`
// whose `subject` is the group.

import Foundation
import WordsModel

public struct GroupMenuFacts: Sendable, Equatable {
    public struct Colour: Sendable, Equatable {
        public var name: String
        /// The theme token the dot is drawn from.
        public var token: String
        public init(name: String, token: String) { (self.name, self.token) = (name, token) }
    }

    public var group: Int64
    public var pinned = false
    /// The named colours on offer, and which one the group wears (nil: none, or a custom one).
    public var colours: [Colour] = []
    public var chosen: Int?
    /// The group wears a colour of the person's own.
    public var custom = false
    /// The group wears one of the named colours, which a menu with no dots of its own leaves unsaid.
    public var coloured = false
    public var folder: String?
    public var onlyInFolder = false

    public init(group: Int64) { self.group = group }
}

public enum GroupMenu {
    public enum Kind: String, Sendable, CaseIterable {
        case rename, colour, otherColour, noColour, setFolder, clearFolder, onlyInFolder
        case newTerminal, ungroup, close, togglePin, moveToWindow
        public var key: String { "group.\(rawValue)" }
    }

    /// What the empty ring and its item are called.
    public static var noColour: String { Words.text("sidebar-menu-group-colour-none", fallback: "No colour") }

    public static func nodes(_ facts: GroupMenuFacts) -> [MenuNode] {
        var menu = ContextMenuList()
        let group = facts.group
        func item(_ title: String, _ kind: Kind, checked: Bool = false, enabled: Bool = true) {
            menu.item(title, kind.key, subject: group, checked: checked, enabled: enabled)
        }
        item(Words.text("sidebar-menu-rename", fallback: "Rename…"), .rename)
        menu.separator()
        let swatches = facts.colours.map { SwatchRow.Swatch(name: $0.name, token: $0.token) }
        if !swatches.isEmpty {
            menu.add(.swatches(SwatchRow(swatches: swatches, selected: facts.chosen) { ContextAction(Kind.colour.key, subject: group, argument: String($0)) }))
        }
        item(Words.text("sidebar-menu-group-colour-other", fallback: "Other colour…"), .otherColour, checked: facts.custom)
        item(noColour, .noColour, checked: facts.chosen == nil && !facts.custom && !facts.coloured)
        menu.separator()
        if let folder = facts.folder {
            menu.item(folder, Kind.setFolder.key, subject: group, enabled: false)
            item(Words.text("sidebar-menu-group-set-folder", fallback: "Set folder…"), .setFolder)
            item(Words.text("sidebar-menu-group-clear-folder", fallback: "Clear folder"), .clearFolder)
            item(Words.text("sidebar-group-only-in-folder", fallback: "Only terminals in this folder"), .onlyInFolder, checked: facts.onlyInFolder)
        } else {
            item(Words.text("sidebar-menu-group-set-folder", fallback: "Set folder…"), .setFolder)
        }
        menu.separator()
        item(Words.text("sidebar-menu-group-new-terminal", fallback: "New terminal in group"), .newTerminal)
        item(Words.text("sidebar-menu-group-ungroup", fallback: "Ungroup"), .ungroup)
        item(Words.text("sidebar-menu-group-close", fallback: "Close group"), .close)
        item(facts.pinned ? Words.text("sidebar-menu-group-unpin", fallback: "Unpin group") : Words.text("sidebar-menu-group-pin", fallback: "Pin group"), .togglePin)
        item(Words.text("sidebar-menu-group-move-to-window", fallback: "Move group to new window"), .moveToWindow)
        return menu.finished
    }
}
