// The terminal's own context menu (`terminal_view/context_menu.rs`): the link under the pointer, Copy, Paste,
// Select All, Find…, the command block's copies, the splits and Clear Buffer. Keys come from the keymap, so the
// menu shows what the menu bar does.

import Foundation
import WordsModel

public struct TerminalMenuFacts: Sendable, Equatable {
    public struct Block: Sendable, Equatable {
        public var id: UInt64
        public var hasCommand: Bool
        public var hasOutput: Bool
        /// The shell waits at its prompt, which is when a command can be typed there.
        public var atPrompt: Bool
        /// The Fold output feature is on and the block has output to fold.
        public var canFold: Bool
        public init(id: UInt64, hasCommand: Bool, hasOutput: Bool, atPrompt: Bool, canFold: Bool = false) {
            (self.id, self.hasCommand, self.hasOutput, self.atPrompt, self.canFold) = (id, hasCommand, hasOutput, atPrompt, canFold)
        }
    }

    public var link: String?
    public var canCopy = false
    public var canPaste = false
    public var block: Block?
    public var splitRight = true
    public var splitDown = true
    public init() {}
}

public enum TerminalMenu {
    public enum Kind: String, Sendable, CaseIterable {
        case openLink, copyLink, copy, paste, selectAll, find, copyCommand, copyOutput, putCommandAtPrompt, foldOutput
        case splitRight, splitDown, clear
        public var key: String { "terminal.\(rawValue)" }
    }

    public static func nodes(_ facts: TerminalMenuFacts, keymap: Keymap = .shared) -> [MenuNode] {
        var menu = ContextMenuList()
        func item(_ message: StaticString, _ english: String, _ kind: Kind, _ symbol: String, _ id: ActionID? = nil, enabled: Bool = true, argument: String? = nil, subject: Int64? = nil) {
            menu.item(Words.text(message, fallback: english), kind.key, subject: subject, argument: argument, enabled: enabled, icon: .symbol(symbol), key: id.flatMap(keymap.key))
        }
        if let link = facts.link {
            item("terminal-menu-open-link", "Open Link", .openLink, "safari", argument: link)
            item("terminal-menu-copy-link", "Copy Link", .copyLink, "link", argument: link)
        }
        menu.separator()
        item("terminal-menu-copy", "Copy", .copy, "doc.on.doc", "copy", enabled: facts.canCopy)
        item("terminal-menu-paste", "Paste", .paste, "doc.on.clipboard", "paste", enabled: facts.canPaste)
        item("terminal-menu-select-all", "Select All", .selectAll, "selection.pin.in.out", "selectAll")
        menu.separator()
        item("terminal-menu-find", "Find…", .find, "magnifyingglass", "find")
        if let block = facts.block {
            menu.separator()
            let subject = Int64(bitPattern: block.id)
            item("terminal-menu-copy-command", "Copy Command", .copyCommand, "terminal", "copyCommand", enabled: block.hasCommand, subject: subject)
            item("terminal-menu-copy-output", "Copy Output", .copyOutput, "text.alignleft", "copyOutput", enabled: block.hasOutput, subject: subject)
            item("terminal-menu-put-command-at-prompt", "Put Command at Prompt", .putCommandAtPrompt, "arrow.turn.down.left", "rerunCommand",
                enabled: block.hasCommand && block.atPrompt, subject: subject)
            if block.canFold { item("terminal-menu-fold-output", "Fold Output", .foldOutput, "rectangle.compress.vertical", subject: subject) }
        }
        menu.separator()
        item("terminal-menu-split-right", "Split Right", .splitRight, "rectangle.split.2x1", "splitRight", enabled: facts.splitRight)
        item("terminal-menu-split-down", "Split Down", .splitDown, "rectangle.split.1x2", "splitDown", enabled: facts.splitDown)
        menu.separator()
        item("terminal-menu-clear", "Clear Buffer", .clear, "eraser", "clearBuffer")
        return menu.finished
    }
}
