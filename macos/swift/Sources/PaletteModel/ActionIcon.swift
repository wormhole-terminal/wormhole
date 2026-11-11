// The icon an action leads with in the palette (core's palette view, `action_icon`: a few have their own, the rest the
// command glyph), as SF Symbols.

import Foundation

enum ActionIcon {
    static func symbol(_ action: String) -> String {
        switch action {
        case "newTerminal", "newWindow", "newNote": "plus"
        case "openFolder": "folder.badge.plus"
        case "showChanges": "plusminus"
        case "showHistory": "clock.arrow.circlepath"
        case "showNotes": "note.text"
        case "showProject": "folder"
        case "showUsage": "gauge"
        case "showFiles": "list.bullet.indent"
        case "toggleSidebar": "sidebar.left"
        case "toggleRightPanel": "sidebar.right"
        case "stashPrompt": "tray"
        case "keyboardShortcuts": "keyboard"
        case "themeStudio": "paintpalette"
        case "settings", "checkForUpdates": "gearshape"
        case "closeTerminal", "closeWindow": "xmark"
        case "pinTerminal": "pin"
        case "hideFromAgents": "lock"
        case "copyPath": "doc.on.doc"
        case "revealInFinder": "arrow.up.right.square"
        default: "command"
        }
    }
}
