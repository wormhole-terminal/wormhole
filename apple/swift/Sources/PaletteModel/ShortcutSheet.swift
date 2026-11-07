// The keyboard-shortcut sheet as data: sections of labelled keys, in core's `keys/keymap/sheet.rs` wording and
// order, for the actions this app has (so a split or a window is not listed). A row's keys are its actions'.

import Foundation
import WorkspaceModel

public struct ShortcutRow: Sendable, Equatable, Identifiable {
    /// How a row's keys are written: each action's key side by side, or the first and last of a run.
    public enum Style: Sendable, Equatable { case each, range }

    public var label: String
    public var keys: [Shortcut]
    public var style: Style = .each
    /// Every key the row stands for, a run's middle ones too: what a key typed in the filter finds.
    public var allKeys: [Shortcut]
    public var id: String { label }

    public init(label: String, keys: [Shortcut], style: Style = .each, allKeys: [Shortcut]? = nil) {
        self.label = label
        self.keys = keys
        self.style = style
        self.allKeys = allKeys ?? keys
    }

    /// The keys as the sheet writes them, which its filter matches: "⌘1 … ⌘9".
    public var keysText: String { keys.map(\.glyphs).joined(separator: style == .range ? " … " : " ") }
}

public struct ShortcutSection: Sendable, Equatable, Identifiable {
    public var title: String
    public var rows: [ShortcutRow]
    public var id: String { title }
}

public enum ShortcutSheet {
    public static let sections: [ShortcutSection] = [
        ShortcutSection(
            title: "Windows & terminals",
            rows: [
                ShortcutRow(label: "New terminal", keys: [Shortcuts.newTerminal]),
                ShortcutRow(label: "Close terminal", keys: [Shortcuts.closeTerminal]),
                ShortcutRow(
                    label: "Previous / next terminal", keys: [Shortcuts.previousTerminal, Shortcuts.nextTerminal]),
                ShortcutRow(
                    label: "Terminals 1–9", keys: [Shortcuts.terminal(number: 1), Shortcuts.terminal(number: 9)],
                    style: .range, allKeys: (1...9).map { Shortcuts.terminal(number: $0) }),
            ]),
        ShortcutSection(
            title: "View & panels",
            rows: [
                ShortcutRow(label: "Hide / show sidebar", keys: [Shortcuts.toggleSidebar]),
                ShortcutRow(label: "Hide / show right panel", keys: [Shortcuts.togglePanel]),
                // Three of the six panel tabs, so not a run: a run with a gap would promise the keys of the missing.
                ShortcutRow(
                    label: "Changes · History · Notes",
                    keys: [.changes, .history, .notes].map(Shortcuts.panelTab)),
            ]),
        ShortcutSection(
            title: "Find anything",
            rows: [
                ShortcutRow(label: "Command palette", keys: [Shortcuts.commandPalette]),
                ShortcutRow(label: "Keyboard shortcuts (this sheet)", keys: [Shortcuts.shortcutSheet]),
            ]),
    ]
}
