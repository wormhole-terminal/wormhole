// A terminal of any window as the palette lists it (core's `command_palette/rows/items.rs` `TerminalItem`): the facts it shows, the
// window it is in and its ⌘ number in the palette's own window. The words under its name are `rows/detail.rs`'s, with the time read when
// asked (`elapsed`, which the window supplies: how long it has been in its state, in the surfaces' words) so a row counts on while
// the palette is open.

import Foundation
import WorkspaceModel

public struct PaletteTerminal: Sendable {
    public var id: TerminalID
    public var name: String
    /// Its window as the title bar numbers it ("Window 2").
    public var window: Int
    public var activity: Activity
    public var number: Int?
    /// Where it is, `~`-shortened.
    public var path: String?
    /// The folder's name, which the words say (core's `notify::Terminal.folder`).
    public var folder: String?
    public var branch: String?
    /// The agent ("Claude Code") or program that tells the state, what it says, the command running, how it exited.
    public var who: String?
    public var message: String?
    public var command: String?
    public var exit: Int?
    /// "now", "4:12", "12m ago" for the state it is in at this time; nil when idle.
    public var elapsed: @Sendable (Date) -> String?

    public init(
        id: TerminalID, name: String, window: Int, activity: Activity, number: Int? = nil, path: String? = nil, folder: String? = nil,
        branch: String? = nil, who: String? = nil, message: String? = nil, command: String? = nil, exit: Int? = nil,
        elapsed: @escaping @Sendable (Date) -> String? = { _ in nil }
    ) {
        (self.id, self.name, self.window, self.activity, self.number, self.path, self.folder) = (id, name, window, activity, number, path, folder)
        (self.branch, self.who, self.message, self.command, self.exit, self.elapsed) = (branch, who, message, command, exit, elapsed)
    }

    /// A window's own terminals when nothing else says more (previews, tests): no clock, folder as the row has it.
    public init(_ summary: TerminalSummary, window: Int = 1, number: Int? = nil) {
        self.init(
            id: summary.id, name: summary.title, window: window, activity: summary.activity, number: number, path: summary.folder,
            folder: summary.folder, branch: summary.branch, who: summary.agent?.name, message: summary.message)
    }

    /// "Window 1 · terminal · 4:12" in the Terminals scope, else what it is doing: "Needs input · Approve cargo test?".
    public func detail(scoped: Bool, now: Date) -> String? {
        let words = scoped ? ["Window \(window)", folder, elapsed(now)].compactMap { $0 }.joined(separator: " · ") : doingWords
        return words.isEmpty ? nil : words
    }

    private var doingWords: String {
        let doing: String? =
            switch activity {
            case .needsInput: "Needs input"
            case .finished: "Finished"
            case .failed: "Failed"
            case .working, .idle, .exited: folder
            }
        let exitWords = exit.map { "exit \($0)" }
        let words: String? =
            switch activity {
            case .needsInput, .working: message ?? command
            // What went wrong: a program's own words, else the command and how it exited.
            case .failed:
                message ?? ([command, exitWords].compactMap { $0 }.joined(separator: " · ")).nilIfEmpty
            default: nil
            }
        return [doing, words].compactMap { $0 }.joined(separator: " · ")
    }

    /// The line under a terminal that waits for an answer, in the Terminals scope.
    public var question: String? {
        guard activity == .needsInput, let message else { return nil }
        return who.map { "\($0) is asking: \(message)" } ?? message
    }
}

private extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}
