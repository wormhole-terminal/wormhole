// The palette's actions that are the window's own verbs rather than workspace actions: splits, find, the pane zoom, the
// secondary windows. Each names the keymap action (`appAction`) the window runs, so the palette, the menu and the key
// all reach the same code; the words are core's `keymap/palette.rs`.

import Foundation

enum AppCommands {
    static func all(hasSelection: Bool) -> [Command] {
        var out: [Command] = []
        if hasSelection {
            out += [
                item("split-right", "Split right", "rectangle.split.2x1", "splitRight", Shortcut(.command, "D")),
                item("split-down", "Split down", "rectangle.split.1x2", "splitDown", Shortcut([.shift, .command], "D")),
                item("zoom-pane", "Zoom pane", "arrow.up.left.and.arrow.down.right", "zoomPane", Shortcut([.shift, .command], "↩")),
                item("find", "Find…", "magnifyingglass", "find", Shortcut(.command, "F")),
                item("copy-path", "Copy path", "doc.on.doc", "copyPath", nil),
                item("reveal", "Reveal in Finder", "folder", "revealInFinder", nil),
            ]
        }
        out += [
            item("go-waiting", "Go to waiting terminal", "bell", "goToWaiting", Shortcut([.option, .command], "J")),
            item("toggle-status-bar", "Toggle status bar", "rectangle.bottomthird.inset.filled", "toggleStatusBar", Shortcut(.command, "/")),
            item("toggle-tabs", "Show terminals as tabs", "rectangle.topthird.inset.filled", "toggleTerminalTabs", nil),
            item("settings", "Open settings", "gearshape", "settings", Shortcut(.command, ",")),
            item("about", "About Wormhole", "info.circle", "about", nil),
            item("check-updates", "Check for updates", "arrow.triangle.2.circlepath", "checkForUpdates", nil),
            item("licenses", "Licenses", "doc.text", "licenses", nil),
            item("session-hosts", "Session hosts", "server.rack", "sessionHosts", nil),
        ]
        return out
    }

    private static func item(_ id: String, _ title: String, _ symbol: String, _ action: String, _ key: Shortcut?) -> Command {
        Command(id: id, title: title, lead: .symbol(symbol), shortcut: key, appAction: action, section: .view)
    }
}
