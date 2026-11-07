// The commands the Swift app has, for the window as it is now: toggles say what they will do ("Hide
// Sidebar"), and every terminal is a "Go to" entry. Wording follows core's keymap table (sentence-case titles).

import Foundation
import WorkspaceModel

public enum CommandCatalog {
    public static func commands(for state: WorkspaceState) -> [Command] {
        var out: [Command] = [
            Command(
                id: "new-terminal", title: "New Terminal", category: .terminals, symbol: "plus.rectangle",
                shortcut: Shortcuts.newTerminal, action: .newTerminal)
        ]
        if let selected = state.terminals.first(where: { $0.id == state.selected }) {
            out += selectedTerminalCommands(selected, state: state)
        }
        out += [
            Command(
                id: "toggle-sidebar", title: state.sidebarVisible ? "Hide Sidebar" : "Show Sidebar", category: .view,
                symbol: "sidebar.left", shortcut: Shortcuts.toggleSidebar, action: .toggleSidebar),
            Command(
                id: "toggle-panel", title: state.panelVisible ? "Hide Right Panel" : "Show Right Panel",
                category: .view, symbol: "sidebar.right", shortcut: Shortcuts.togglePanel, action: .togglePanel),
        ]
        for tab in [PanelTab.changes, .history, .notes] {
            out.append(
                Command(
                    id: "panel-\(tab.rawValue)", title: "Show \(tab.rawValue.capitalized)", category: .view,
                    symbol: symbol(for: tab), shortcut: Shortcuts.panelTab(tab), action: .showPanelTab(tab)))
        }
        for (index, terminal) in state.terminals.enumerated() {
            out.append(
                Command(
                    id: "go-\(terminal.id)", title: "Go to \(terminal.title)", subtitle: detail(of: terminal),
                    group: .terminals, category: .terminals, symbol: "terminal",
                    shortcut: index < 9 ? Shortcuts.terminal(number: index + 1) : nil, action: .select(terminal.id)))
        }
        return out
    }

    /// Entries the window handles itself (it opens the overlays), so they are not runnable from the palette.
    public static let windowEntries: [Command] = [
        Command(
            id: "command-palette", title: "Command Palette", category: .findAnything, symbol: "command",
            shortcut: Shortcuts.commandPalette),
        Command(
            id: "shortcut-sheet", title: "Keyboard Shortcuts", category: .findAnything, symbol: "keyboard",
            shortcut: Shortcuts.shortcutSheet),
    ]

    private static func selectedTerminalCommands(_ terminal: TerminalSummary, state: WorkspaceState) -> [Command] {
        var out = [
            Command(
                id: "close-terminal", title: "Close Terminal", category: .terminals, symbol: "xmark.rectangle",
                shortcut: Shortcuts.closeTerminal, action: .close(terminal.id)),
            Command(
                id: "pin-terminal", title: terminal.isPinned ? "Unpin Terminal" : "Pin to Top",
                category: .terminals, symbol: terminal.isPinned ? "pin.slash" : "pin",
                action: .togglePin(terminal.id)),
        ]
        guard let at = state.terminals.firstIndex(where: { $0.id == terminal.id }), state.terminals.count > 1 else {
            return out
        }
        let count = state.terminals.count
        out += [
            Command(
                id: "next-terminal", title: "Next Terminal", category: .terminals, symbol: "chevron.down",
                shortcut: Shortcuts.nextTerminal, action: .select(state.terminals[(at + 1) % count].id)),
            Command(
                id: "previous-terminal", title: "Previous Terminal", category: .terminals, symbol: "chevron.up",
                shortcut: Shortcuts.previousTerminal, action: .select(state.terminals[(at + count - 1) % count].id)),
        ]
        return out
    }

    private static func detail(of terminal: TerminalSummary) -> String? {
        let parts = [terminal.subtitle, terminal.branch].compactMap { $0 }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    private static func symbol(for tab: PanelTab) -> String {
        switch tab {
        case .changes: "plusminus"
        case .history: "clock.arrow.circlepath"
        case .notes: "note.text"
        }
    }
}
