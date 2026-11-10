// What the palette can list for the window as it is now. Actions are the core's keymap as `KeymapLists` gives it for
// this window (its words, order, groups and keys), settings and commits come from the sources the window hands in, and
// terminals and the folders they are in are results of their own.

import Foundation
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
        commits: [PaletteCommit] = [], profiles: [PaletteProfile] = []
    ) -> [Command] {
        self.actions(actions, for: state) + profileCommands(profiles) + settings.map(command) + commits.map(command)
            + terminals(in: state) + folders(in: state)
    }

    /// With another profile to go to: a row to switch to each profile that is not shown, to open a window in each, and to
    /// move this window to each it is not in (core's palette, in that order).
    private static func profileCommands(_ profiles: [PaletteProfile]) -> [Command] {
        guard profiles.count > 1 else { return [] }
        let kinds: [(Command.ProfileCommand, String, String, (PaletteProfile) -> Bool, String?)] = [
            (.switchTo, "Switch to", "person.crop.circle", { !$0.isShown }, "every window switches"),
            (.openWindowIn, "Open window in", "macwindow", { _ in true }, nil),
            (.moveWindowTo, "Move this window to", "arrow.left.arrow.right", { !$0.isHere }, nil),
        ]
        return kinds.flatMap { kind, words, symbol, offered, detail in
            profiles.enumerated().filter { offered($0.element) }.map { index, profile in
                Command(
                    id: "profile-\(words)-\(index)", title: "\(words) \(profile.name)", detail: detail, lead: .symbol(symbol),
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
        case "newTerminal": selected?.folder.map { "in \($0)" }
        case "openFolder": "start a terminal in another folder"
        case "showChanges":
            selected.flatMap { $0.repo == nil ? nil : "\($0.changeCount) file\($0.changeCount == 1 ? "" : "s") changed" }
        default: nil
        }
    }

    private static func command(_ setting: PaletteSetting) -> Command {
        Command(
            id: "setting-\(setting.id)", title: setting.label, detail: "Settings › \(setting.section)", group: .settings,
            lead: .symbol(setting.icon), trailing: setting.value, target: .setting(setting.id), match: setting.match)
    }

    private static func command(_ commit: PaletteCommit) -> Command {
        var command = Command(
            id: "commit-\(commit.sha)", title: commit.subject, group: .commits, lead: .symbol("smallcircle.filled.circle"),
            trailing: commit.short, target: .commit(commit.sha), searchText: [commit.short, commit.author])
        command.trailingIsCode = true
        return command
    }

    private static func terminals(in state: WorkspaceState) -> [Command] {
        state.all.enumerated().map { index, terminal in
            let folder = terminal.folder
            return Command(
                id: "terminal-\(terminal.id)", title: terminal.title,
                detail: doing(terminal), scopedDetail: (["Window 1"] + [folder].compactMap { $0 }).joined(separator: " · "),
                group: .terminals, lead: .status(terminal.activity),
                shortcut: index < 9 ? Shortcut(.command, String(index + 1)) : nil, action: .select(terminal.id),
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
