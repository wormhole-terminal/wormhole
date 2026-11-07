// The Swift app's keys, one place, as the Rust keymap has them (`keys/keymap/table.rs`), so the palette's
// keycaps and the shortcut sheet cannot disagree.

import Foundation
import WorkspaceModel

public enum Shortcuts {
    public static let newTerminal = Shortcut(.command, "T")
    public static let closeTerminal = Shortcut(.command, "W")
    public static let previousTerminal = Shortcut([.control, .command], "↑")
    public static let nextTerminal = Shortcut([.control, .command], "↓")
    public static let toggleSidebar = Shortcut([.control, .command], "S")
    public static let togglePanel = Shortcut([.option, .command], "I")
    public static let commandPalette = Shortcut([.shift, .command], "P")
    public static let shortcutSheet = Shortcut([.option, .command], "/")

    /// ⌘1–⌘9, the first nine rows of the sidebar.
    public static func terminal(number: Int) -> Shortcut { Shortcut(.command, String(number)) }

    /// ⌃⌘1 Changes, ⌃⌘2 History, ⌃⌘3 Notes, ⌃⌘4 Usage, ⌃⌘5 Project, ⌃⌘6 Files: the digits follow the Rust app's tab order, not `PanelTab`'s.
    public static func panelTab(_ tab: PanelTab) -> Shortcut {
        let digit =
            switch tab {
            case .changes: "1"
            case .history: "2"
            case .notes: "3"
            case .usage: "4"
            case .project: "5"
            case .files: "6"
            }
        return Shortcut([.control, .command], digit)
    }
}
