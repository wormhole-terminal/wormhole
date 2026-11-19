// What the palette can list for the window as it is now. Actions are the core's keymap as `KeymapLists` gives it for
// this window (its words, order, groups and keys), settings and commits come from the sources the window hands in, and
// terminals and the folders they are in are results of their own.

import Foundation
import WordsModel
import WorkspaceModel

/// A setting the settings index found (or a recent one): where it is, what it is now.
public struct PaletteSetting: Sendable, Equatable {
    public var id: String
    /// The config key the row sets: what ⌘C copies.
    public var key: String
    public var label: String
    public var section: String
    /// An SF Symbol's name: the section's icon.
    public var icon: String
    /// Its value in the words of its row ("On", "80 %"); nil says nothing.
    public var value: String?
    /// The index's own match (it also reads keywords and descriptions); nil for a recent one, which nothing found.
    public var match: Match?

    public init(id: String, key: String, label: String, section: String, icon: String, value: String?, match: Match? = nil) {
        (self.id, self.key, self.label, self.section, self.icon, self.value, self.match) = (id, key, label, section, icon, value, match)
    }
}

/// A commit of the selected terminal's branch.
public struct PaletteCommit: Sendable, Equatable {
    public var sha: String
    public var short: String
    public var subject: String
    public var author: String

    public init(sha: String, short: String, subject: String, author: String) {
        (self.sha, self.short, self.subject, self.author) = (sha, short, subject, author)
    }
}

/// A profile Wormhole can switch to, open a window in or move the window to: where it is in the list and what it is.
public struct PaletteProfile: Sendable, Equatable {
    public var name: String
    /// The one Wormhole is in.
    public var isShown: Bool
    /// The one this window belongs to.
    public var isHere: Bool

    public init(name: String, isShown: Bool, isHere: Bool) { (self.name, self.isShown, self.isHere) = (name, isShown, isHere) }
}

public enum CommandCatalog {
    public static func commands(
        for state: WorkspaceState, actions: ActionList = ActionList(), settings: [PaletteSetting] = [],
        commits: [PaletteCommit] = [], profiles: [PaletteProfile] = [], terminals: [PaletteTerminal] = [],
        project: PaletteProject? = nil, shells: [PaletteShell] = [], recentFolders: [String] = [], now: Date = Date()
    ) -> [Command] {
        let listed = terminals.isEmpty ? state.all.enumerated().map { PaletteTerminal($1, number: $0 < 9 ? $0 + 1 : nil) } : terminals
        // The Actions group in core's order: the table's, the project's, the shells', the profiles'.
        return self.actions(actions, for: state) + (project?.actions.map(command) ?? []) + shells.map(command)
            + profileCommands(profiles) + settings.map(command) + commits.map(command) + self.terminals(listed, now: now)
            + folders(of: listed, recent: recentFolders)
    }

    private static func command(_ action: PaletteProject.Action) -> Command {
        var command = Command(
            id: "project-\(action.id)", title: action.title, detail: action.detail, lead: .symbol("bolt"),
            shortcut: action.key.flatMap { Shortcut(glyphs: $0) }, target: .projectAction(action.id))
        command.detailIsCode = true
        return command
    }

    /// A terminal with another shell: named like New terminal, found by what it runs too.
    private static func command(_ shell: PaletteShell) -> Command {
        var command = Command(
            id: "shell-\(shell.key)", title: Words.text("palette-new-terminal-with", ["shell": .text(shell.name)], fallback: "New terminal: \(shell.name)"), detail: shell.commandLine, lead: .symbol("plus"),
            target: .newTerminalWith(shell.key), searchText: [shell.commandLine], section: .terminal)
        command.detailIsCode = true
        return command
    }

    /// With another profile to go to: a row to switch to each profile that is not shown, to open a window in each, and to
    /// move this window to each it is not in (core's palette, in that order).
    private static func profileCommands(_ profiles: [PaletteProfile]) -> [Command] {
        guard profiles.count > 1 else { return [] }
        let kinds: [(Command.ProfileCommand, (String) -> String, String, (PaletteProfile) -> Bool, String?)] = [
            (
                .switchTo, { Words.text("palette-profile-switch", ["profile": .text($0)], fallback: "Switch to \($0)") },
                "person.crop.circle", { !$0.isShown }, Words.text("palette-profile-switch-detail", fallback: "every window switches")
            ),
            (
                .openWindowIn, { Words.text("palette-profile-open-window", ["profile": .text($0)], fallback: "Open window in \($0)") },
                "macwindow", { _ in true }, nil
            ),
            (
                .moveWindowTo, { Words.text("palette-profile-move-window", ["profile": .text($0)], fallback: "Move this window to \($0)") },
                "arrow.left.arrow.right", { !$0.isHere }, nil
            ),
        ]
        return kinds.flatMap { kind, title, symbol, offered, detail in
            profiles.enumerated().filter { offered($0.element) }.map { index, profile in
                Command(
                    id: "profile-\(kind)-\(index)", title: title(profile.name), detail: detail, lead: .symbol(symbol),
                    target: .profile(kind, index), section: .profiles)
            }
        }
    }

    static func actions(_ list: ActionList, for state: WorkspaceState) -> [Command] {
        let selected = state.all.first { $0.id == state.selected }
        return list.rows.map { row in
            Command(
                id: "action-\(row.id)", title: row.label, detail: detail(of: row.id, selected: selected),
                lead: .symbol(ActionIcon.symbol(row.id)), shortcut: row.key, appAction: row.id, section: row.section)
        }
    }

    /// The line after an action's name, where the window knows more (core's palette view): where a new terminal opens,
    /// how many files changed.
    private static func detail(of action: String, selected: TerminalSummary?) -> String? {
        switch action {
        case "newTerminal": selected?.folder.map { Words.text("palette-detail-new-terminal-in", ["folder": .text($0)], fallback: "in \($0)") }
        case "openFolder": Words.text("palette-detail-open-folder", fallback: "start a terminal in another folder")
        case "showChanges":
            selected.flatMap {
                $0.repo == nil
                    ? nil
                    : Words.text(
                        "palette-detail-files-changed", ["count": .whole($0.changeCount)],
                        fallback: "\($0.changeCount) file\($0.changeCount == 1 ? "" : "s") changed")
            }
        default: nil
        }
    }

    private static func command(_ setting: PaletteSetting) -> Command {
        Command(
            id: "setting-\(setting.id)", title: setting.label, detail: Words.text("palette-detail-setting", ["section": .text(setting.section)], fallback: "Settings › \(setting.section)"), group: .settings,
            lead: .symbol(setting.icon), trailing: setting.value, target: .setting(setting.id), match: setting.match)
    }

    private static func command(_ commit: PaletteCommit) -> Command {
        var command = Command(
            id: "commit-\(commit.sha)", title: commit.subject, group: .commits, lead: .symbol("smallcircle.filled.circle"),
            trailing: commit.short, target: .commit(commit.sha), searchText: [commit.short, commit.author])
        command.trailingIsCode = true
        return command
    }

    private static func terminals(_ listed: [PaletteTerminal], now: Date) -> [Command] {
        listed.map { item in
            Command(
                id: "terminal-\(item.id)", title: item.name, detail: item.detail(scoped: false, now: now),
                scopedDetail: item.detail(scoped: true, now: now), second: item.question, group: .terminals,
                lead: .status(item.activity, told: item.told), shortcut: item.number.map { Shortcut(.command, String($0)) },
                target: .terminal(item.id), isStrong: true,
                searchText: [item.folder, item.branch, item.command, item.who, item.message].compactMap { $0 })
        }
    }

    /// The folders terminals are in, with how many each holds. A folder that is a program, not a path, is not one.
    private static func folders(of listed: [PaletteTerminal], recent: [String]) -> [Command] {
        var order: [String] = []
        var counts: [String: Int] = [:]
        for item in listed {
            guard let path = item.path, path.hasPrefix("~") || path.hasPrefix("/") else { continue }
            if counts[path] == nil { order.append(path) }
            counts[path, default: 0] += 1
        }
        // The projects worked in lately that have no terminal now follow, with their parent only (core `gather`).
        let idle = recent.map { ($0 as NSString).abbreviatingWithTildeInPath }.filter { counts[$0] == nil }
        order += idle
        return order.map { path in
            let name = path.split(separator: "/").last.map(String.init) ?? path
            let parent = path.split(separator: "/").dropLast().joined(separator: "/")
            let count = counts[path] ?? 0
            let origin = parent.isEmpty ? nil : (path.hasPrefix("/") ? "/" : "") + parent
            return Command(
                id: "folder-\(path)", title: name,
                detail: [origin, count == 0 ? nil : Words.text("palette-detail-terminals", ["count": .whole(count)], fallback: "\(count) terminal\(count == 1 ? "" : "s")")].compactMap { $0 }.joined(separator: " · "),
                group: .folders, lead: .symbol("folder"), target: .folder(path))
        }
    }
}
