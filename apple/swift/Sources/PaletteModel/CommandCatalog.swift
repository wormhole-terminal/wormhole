// What the palette can list for the window as it is now, worded as core's `keymap/palette.rs` words the
// actions: sentence case, a toggle by what it will do ("Hide sidebar"), listed most-used first. Terminals and the
// folders they are in are results of their own. Settings and commits have no source in this app yet, so
// those scopes stay empty, as they do in a Wormhole window outside a repository.

import Foundation
import WorkspaceModel

public enum CommandCatalog {
    /// What an empty query suggests under "Actions", in this order (core `palette::SUGGESTED`).
    public static let suggested = ["new-terminal", "panel-changes", "toggle-panel", "keyboard-shortcuts"]

    public static func commands(for state: WorkspaceState) -> [Command] {
        actions(for: state) + terminals(in: state) + folders(in: state)
    }

    static func actions(for state: WorkspaceState) -> [Command] {
        let selected = state.all.first { $0.id == state.selected }
        var out = [command("new-terminal", "New terminal", "plus", .newTerminal, Shortcuts.newTerminal, .terminal)]
        out[0].detail = selected?.folder.map { "in \($0)" }
        if let selected {
            out.append(command("close-terminal", "Close terminal", "xmark", .close(selected.id), Shortcuts.closeTerminal, .terminal))
        }
        out += [
            command(
                "toggle-sidebar", state.sidebarVisible ? "Hide sidebar" : "Show sidebar", "sidebar.left",
                .toggleSidebar, Shortcuts.toggleSidebar, .view),
            command(
                "panel-changes", "Show changes", "plusminus", .showPanelTab(.changes), Shortcuts.panelTab(.changes), .view),
            command(
                "panel-history", "Show history", "clock.arrow.circlepath", .showPanelTab(.history),
                Shortcuts.panelTab(.history), .view),
            command(
                "toggle-panel", state.panelVisible ? "Hide right panel" : "Show right panel", "sidebar.right",
                .togglePanel, Shortcuts.togglePanel, .view),
            command("panel-notes", "Show notes", "note.text", .showPanelTab(.notes), Shortcuts.panelTab(.notes), .view),
            command("panel-usage", "Show usage", "gauge", .showPanelTab(.usage), Shortcuts.panelTab(.usage), .view),
            command("panel-project", "Show project", "folder.badge.gearshape", .showPanelTab(.project), Shortcuts.panelTab(.project), .view),
            command("panel-files", "Show files", "folder", .showPanelTab(.files), Shortcuts.panelTab(.files), .view),
            command(
                "keyboard-shortcuts", "Keyboard shortcuts", "keyboard", .showOverlay(.shortcuts),
                Shortcuts.shortcutSheet, .view),
        ]
        if let selected { out += neighbours(of: selected, in: state) + AppCommands.all(hasSelection: true) } else { out += AppCommands.all(hasSelection: false) }
        return out
    }

    private static func neighbours(of terminal: TerminalSummary, in state: WorkspaceState) -> [Command] {
        var out = [
            command(
                "pin-terminal", terminal.isPinned ? "Unpin terminal" : "Pin to top", "pin", .togglePin(terminal.id), nil,
                .terminal)
        ]
        guard let at = state.all.firstIndex(where: { $0.id == terminal.id }), state.all.count > 1 else {
            return out
        }
        let count = state.all.count
        out.append(
            command(
                "next-terminal", "Next terminal", "command", .select(state.all[(at + 1) % count].id),
                Shortcuts.nextTerminal, .terminal))
        out.append(
            command(
                "previous-terminal", "Previous terminal", "command", .select(state.all[(at + count - 1) % count].id),
                Shortcuts.previousTerminal, .terminal))
        return out
    }

    private static func command(
        _ id: String, _ title: String, _ symbol: String, _ action: WorkspaceAction, _ key: Shortcut?,
        _ section: ActionSection
    ) -> Command {
        Command(id: id, title: title, lead: .symbol(symbol), shortcut: key, action: action, section: section)
    }

    private static func terminals(in state: WorkspaceState) -> [Command] {
        state.all.enumerated().map { index, terminal in
            let folder = terminal.folder
            return Command(
                id: "terminal-\(terminal.id)", title: terminal.title,
                detail: doing(terminal), scopedDetail: (["Window 1"] + [folder].compactMap { $0 }).joined(separator: " · "),
                group: .terminals, lead: .status(terminal.activity),
                shortcut: index < 9 ? Shortcuts.terminal(number: index + 1) : nil, action: .select(terminal.id),
                isStrong: true, searchText: [terminal.folder, terminal.branch].compactMap { $0 })
        }
    }

    /// What a terminal is doing, in the words of core's `terminal_detail`: a state worth saying, else its folder.
    private static func doing(_ terminal: TerminalSummary) -> String? {
        switch terminal.activity {
        case .needsInput: "Needs input"
        case .finished: "Finished"
        case .failed: "Failed"
        case .working, .idle, .exited: terminal.folder
        }
    }

    /// The folders terminals are in, with how many each holds. A folder that is a program, not a path, is not one.
    private static func folders(in state: WorkspaceState) -> [Command] {
        var order: [String] = []
        var counts: [String: Int] = [:]
        for terminal in state.all {
            guard let path = terminal.folder, path.hasPrefix("~") || path.hasPrefix("/") else { continue }
            if counts[path] == nil { order.append(path) }
            counts[path, default: 0] += 1
        }
        return order.map { path in
            let name = path.split(separator: "/").last.map(String.init) ?? path
            let parent = path.split(separator: "/").dropLast().joined(separator: "/")
            let count = counts[path] ?? 0
            let origin = parent.isEmpty ? nil : (path.hasPrefix("/") ? "/" : "") + parent
            return Command(
                id: "folder-\(path)", title: name,
                detail: [origin, "\(count) terminal\(count == 1 ? "" : "s")"].compactMap { $0 }.joined(separator: " · "),
                group: .folders, lead: .symbol("folder"),
                action: state.all.first { $0.folder == path }.map { .select($0.id) })
        }
    }
}
