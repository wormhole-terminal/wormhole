// The status item's menu for a snapshot (`menubar::model::build`): the terminals by status, New Terminal and
// Open Folder, the projects, then Show Wormhole, Jump to Next Waiting, Settings, updates and Quit.

import Foundation

/// What the menu is built from: every terminal of every window and the project facts.
public struct StatusSnapshot: Sendable, Equatable {
    public struct Action: Sendable, Equatable {
        public var project: String, root: String, id: String, title: String
        public init(project: String, root: String, id: String, title: String) {
            (self.project, self.root, self.id, self.title) = (project, root, id, title)
        }
    }

    public struct Recent: Sendable, Equatable {
        public var name: String, path: String
        public init(name: String, path: String) { (self.name, self.path) = (name, path) }
    }

    public var terminals: [AmbientTerminal] = []
    public var actions: [Action] = []
    public var recent: [Recent] = []
    /// The global hotkey, shown beside Show Wormhole, and Jump to Next Waiting's key.
    public var hotkey: KeySpec?
    public var jumpKey: KeySpec?
    public var icon: MenuBarIconMode = .statusCount
    public var animate = false

    public init() {}
}

public struct StatusMenuModel: Sendable, Equatable {
    public var icon: StatusIcon
    public var menu: [MenuNode]
}

public enum StatusMenu {
    /// Terminals listed in the sections before the rest fold into "N more…", so the menu never outgrows the screen.
    public static let maxSessions = 20

    public static func build(_ snapshot: StatusSnapshot, now: Date, keymap: Keymap = .bundled) -> StatusMenuModel {
        let terminals = snapshot.terminals
        let sections = AmbientSections(terminals)
        var nodes: [MenuNode] = []
        var listed = 0, hidden = 0
        for (title, list) in [("Needs input", sections.needsYou), ("Working", sections.working), ("Finished", sections.finished)]
        where !list.isEmpty {
            nodes.append(.header(title))
            for index in list {
                if listed < maxSessions {
                    nodes.append(row(terminals[index], now))
                    listed += 1
                } else {
                    hidden += 1
                }
            }
        }
        if hidden > 0 { nodes.append(.item(MenuItem(.ambient(.showApp), "\(hidden) more…"))) }
        if terminals.isEmpty { nodes.append(.item(MenuItem(.ambient(.showApp), "No terminals open", enabled: false))) }
        if !sections.other.isEmpty {
            if !nodes.isEmpty { nodes.append(.separator) }
            var others = sections.other.prefix(maxSessions).map { row(terminals[$0], now) }
            if sections.other.count > maxSessions {
                others.append(.item(MenuItem(.ambient(.showApp), "\(sections.other.count - maxSessions) more…")))
            }
            nodes.append(.submenu(title: "Other Terminals", items: others, trailing: String(sections.other.count)))
        }
        nodes += [.separator, key(.ambient(.newTerminal), "New Terminal", .newTerminal, keymap), key(.ambient(.openFolder), "Open Folder…", .openFolder, keymap)]
        if !snapshot.recent.isEmpty || !snapshot.actions.isEmpty {
            nodes.append(.submenu(title: "Projects", items: projects(snapshot, keymap)))
        }
        nodes += [
            .separator,
            .item(MenuItem(.ambient(.showApp), "Show Wormhole", key: snapshot.hotkey)),
            .item(MenuItem(.ambient(.jumpToWaiting), "Jump to Next Waiting", key: snapshot.jumpKey, enabled: !sections.needsYou.isEmpty)),
            key(.ambient(.openSettings), "Settings…", .settings, keymap),
            .item(MenuItem(.ambient(.checkForUpdates), "Check for Updates…")),
            .separator,
            key(.ambient(.quit), "Quit Wormhole", .quit, keymap),
        ]
        return StatusMenuModel(icon: StatusIcon(terminals, mode: snapshot.icon, animate: snapshot.animate), menu: nodes)
    }

    private static func key(_ command: MenuCommand, _ title: String, _ id: ActionID, _ keymap: Keymap) -> MenuNode {
        .item(MenuItem(command, title, key: keymap.key(id)))
    }

    static func row(_ terminal: AmbientTerminal, _ now: Date) -> MenuNode {
        .item(MenuItem(
            .ambient(.openTab(terminal.tab)), terminal.name.trimmingCharacters(in: .whitespaces).isEmpty ? "Untitled" : terminal.name,
            icon: .status(StatusGlyph(terminal)), trailing: AmbientWording.elapsed(terminal.state, since: max(0, now.timeIntervalSince(terminal.since))),
            subtitle: terminal.detail(withAgent: true)))
    }
}
