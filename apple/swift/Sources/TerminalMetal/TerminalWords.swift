// The words this module shows, one table: the catalog's keys (`wormhole-text` `locales/en/terminal.ftl`,
// named as Fluent names them) with their English, until the catalogs come through the seam.

import Foundation

enum TerminalWord: String {
    case menuCopy = "terminal-menu-copy"
    case menuPaste = "terminal-menu-paste"
    case menuSelectAll = "terminal-menu-select-all"
    case menuFind = "terminal-menu-find"
    case menuOpenLink = "terminal-menu-open-link"
    case menuCopyLink = "terminal-menu-copy-link"
    case menuSplitRight = "terminal-menu-split-right"
    case menuSplitDown = "terminal-menu-split-down"
    case menuClear = "terminal-menu-clear"
    case accessibilityLabel = "terminal-accessibility-label"
    case scrollbarLabel = "terminal-accessibility-scrollback"

    var text: String { NSLocalizedString(rawValue, value: english, comment: "") }

    private var english: String {
        switch self {
        case .menuCopy: "Copy"
        case .menuPaste: "Paste"
        case .menuSelectAll: "Select All"
        case .menuFind: "Find…"
        case .menuOpenLink: "Open Link"
        case .menuCopyLink: "Copy Link"
        case .menuSplitRight: "Split Right"
        case .menuSplitDown: "Split Down"
        case .menuClear: "Clear Buffer"
        case .accessibilityLabel: "Terminal"
        case .scrollbarLabel: "Scrollback"
        }
    }
}
