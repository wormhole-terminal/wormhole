// A terminal row's context menu, in a browser's words (`sidebar/row_menu.rs`): Rename…, groups, Split right,
// Close terminal; Show beside…, Remove from split, Pin to top, the text and agent switches; Reveal in Finder, Copy
// path. The facts are as they are when the menu opens; choosing an item answers with a `ContextAction` whose
// `kind` is one of `RowMenu.Kind` and whose `subject` is the terminal.

import Foundation
import WordsModel

public struct RowMenuFacts: Sendable, Equatable {
    public struct Group: Sendable, Equatable {
        public var id: Int64
        public var name: String
        /// The terminal is in it.
        public var current: Bool
        /// It takes the terminal: a group that keeps only the terminals in its folder doesn't take one from outside.
        public var allowed: Bool
        public init(id: Int64, name: String, current: Bool = false, allowed: Bool = true) {
            (self.id, self.name, self.current, self.allowed) = (id, name, current, allowed)
        }
    }

    public struct Neighbour: Sendable, Equatable {
        public var id: Int64
        public var name: String
        public init(id: Int64, name: String) { (self.id, self.name) = (id, name) }
    }

    public var id: Int64
    public var pinned = false
    /// Its text isn't kept for a restart.
    public var forgetText = false
    /// "Hide from Agents" is offered, ticked when the terminal is hidden from them.
    public var agentHide: Bool?
    /// Restore terminal text after restart is on.
    public var restoring = true
    public var inSplit = false
    public var cwd: String?
    public var groups: [Group] = []
    /// The terminals it could show beside this one.
    public var beside: [Neighbour] = []

    public init(id: Int64) { self.id = id }
}

public enum RowMenu {
    public enum Kind: String, Sendable, CaseIterable {
        case rename, newGroup, intoGroup, removeFromGroup, splitRight, close, showBeside, removeFromSplit
        case togglePin, toggleForgetText, toggleAgentHidden, revealInFinder, copyPath
        public var key: String { "row.\(rawValue)" }
    }

    public static func nodes(_ facts: RowMenuFacts) -> [MenuNode] {
        var menu = ContextMenuList()
        let id = facts.id
        func item(_ title: String, _ kind: Kind, checked: Bool = false, enabled: Bool = true) {
            menu.item(title, kind.key, subject: id, checked: checked, enabled: enabled)
        }
        item(Words.text("sidebar-menu-rename", fallback: "Rename…"), .rename)
        item(Words.text("sidebar-menu-add-to-new-group", fallback: "Add terminal to new group"), .newGroup)
        var into = ContextMenuList()
        for group in facts.groups {
            into.item(group.name, Kind.intoGroup.key, subject: id, argument: String(group.id), checked: group.current, enabled: group.allowed)
        }
        menu.submenu(Words.text("sidebar-menu-add-to-group", fallback: "Add terminal to group"), into)
        if facts.groups.contains(where: \.current) { item(Words.text("sidebar-menu-remove-from-group", fallback: "Remove from group"), .removeFromGroup) }
        item(Words.text("sidebar-menu-split-right", fallback: "Split right"), .splitRight)
        item(Words.text("sidebar-close-terminal", fallback: "Close terminal"), .close)
        menu.separator()
        var next = ContextMenuList()
        for other in facts.beside { next.item(other.name, Kind.showBeside.key, subject: id, argument: String(other.id)) }
        menu.submenu(Words.text("sidebar-menu-show-beside", fallback: "Show beside…"), next)
        if facts.inSplit { item(Words.text("sidebar-menu-remove-from-split", fallback: "Remove from split"), .removeFromSplit) }
        item(facts.pinned ? Words.text("sidebar-unpin", fallback: "Unpin") : Words.text("sidebar-menu-pin-top", fallback: "Pin to top"), .togglePin)
        item(Words.text("sidebar-menu-forget-text", fallback: "Don't keep this terminal's text"), .toggleForgetText, checked: facts.forgetText, enabled: facts.restoring)
        if let hidden = facts.agentHide { item(Words.text("sidebar-menu-hide-from-agents", fallback: "Hide from Agents"), .toggleAgentHidden, checked: hidden) }
        menu.separator()
        item(Words.text("history-menu-reveal", fallback: "Reveal in Finder"), .revealInFinder, enabled: facts.cwd != nil)
        item(Words.text("title-row-path-copy", fallback: "Copy path"), .copyPath, enabled: facts.cwd != nil)
        return menu.finished
    }
}
