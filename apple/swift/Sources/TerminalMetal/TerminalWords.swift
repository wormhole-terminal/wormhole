// The words this module shows, one table: the catalog's keys (`wormhole-text` `locales/en/terminal.ftl`,
// named as Fluent names them) with their English as the fallback of the words seam.

import Foundation
import WordsModel

enum TerminalWord {
    case menuCopy, menuPaste, menuSelectAll, menuFind, menuOpenLink, menuCopyLink, menuSplitRight, menuSplitDown, menuClear
    case accessibilityLabel, scrollbarLabel

    var text: String {
        switch self {
        case .menuCopy: Words.text("terminal-menu-copy", fallback: "Copy")
        case .menuPaste: Words.text("terminal-menu-paste", fallback: "Paste")
        case .menuSelectAll: Words.text("terminal-menu-select-all", fallback: "Select All")
        case .menuFind: Words.text("terminal-menu-find", fallback: "Find…")
        case .menuOpenLink: Words.text("terminal-menu-open-link", fallback: "Open Link")
        case .menuCopyLink: Words.text("terminal-menu-copy-link", fallback: "Copy Link")
        case .menuSplitRight: Words.text("terminal-menu-split-right", fallback: "Split Right")
        case .menuSplitDown: Words.text("terminal-menu-split-down", fallback: "Split Down")
        case .menuClear: Words.text("terminal-menu-clear", fallback: "Clear Buffer")
        case .accessibilityLabel: Words.text("terminal-accessibility-label", fallback: "Terminal")
        case .scrollbarLabel: Words.text("terminal-accessibility-scrollback", fallback: "Scrollback")
        }
    }
}
