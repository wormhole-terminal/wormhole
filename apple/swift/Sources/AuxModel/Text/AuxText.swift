// The words of the secondary windows. The GPUI app keeps its words in `wormhole-text`'s Fluent catalogs; until the
// FFI delivers that table, each message here is one function whose key is the Fluent key, with the English
// source as its default, so a `.strings` table can replace the defaults without touching a caller.

import Foundation

public enum AuxText {
    static func tr(_ key: StaticString, _ value: String.LocalizationValue) -> String {
        String(localized: key, defaultValue: value, table: "Aux")
    }

    static func plural(_ n: Int, one: String, other: String) -> String { n == 1 ? one : other }

    /// The product's name in the catalogs' `-brand` term.
    public static let brand = "Wormhole"
}

// MARK: Questions (dialog.ftl, terminal.ftl, the window's own titles)
extension AuxText {
    public static var cancel: String { tr("dialog-cancel", "Cancel") }
    public static var quit: String { tr("dialog-quit", "Quit") }
    public static var quitAndEndAll: String { tr("dialog-quit-end-all", "Quit and End All") }
    public static var quitTitle: String { tr("dialog-quit-title", "Quit Wormhole?") }
    public static var quitEndAllTitle: String { tr("dialog-quit-end-all-title", "Quit and End All Terminals?") }
    public static var keepRunning: String { tr("dialog-keep-running", "Keep terminals running in the background") }
    public static var quitEnds: String {
        tr("dialog-quit-ends", "Your terminals come back next time, but what runs in them ends.")
    }
    public static var quitKeepsAll: String {
        tr(
            "dialog-quit-keeps-all",
            "Your terminals come back next time and what runs in them keeps going. Untick the box to end them.")
    }

    public static func quitKeepsSome(kept: Int, ended: Int) -> String {
        let keeps = plural(kept, one: "1 terminal keeps", other: "\(kept) terminals keep")
        let ends = plural(ended, one: "1 terminal ends because it runs", other: "\(ended) terminals end because they run")
        return "Your terminals come back next time. \(keeps) running; \(ends) inside \(brand). Untick the box to end the others too."
    }

    public static func quitOtherProfile(count: Int, profile: String) -> String {
        plural(
            count, one: "1 terminal in \(profile) is running an agent or a command.",
            other: "\(count) terminals in \(profile) are running an agent or a command.")
    }

    public static var agentWaiting: String { tr("close-agent-waiting", "An agent is waiting for an answer.") }
    public static var agentWorking: String { tr("close-agent-working", "An agent is still working.") }
    public static var commandRunning: String { tr("close-command-running", "A command is still running.") }
    public static func stillRunning(_ list: String) -> String { tr("close-still-running", "Still running: \(list).") }
    public static func isStillWorking(_ who: String) -> String { tr("close-is-working", "\(who) is still working.") }
    public static func isWaiting(_ who: String) -> String { tr("close-is-waiting", "\(who) is waiting for an answer.") }
    public static var shellEnds: String { tr("close-shell-ends", "Its shell ends.") }
    public static var closingEndsProcess: String { tr("close-ends-process", "Closing ends the process.") }
    public static var closingEndsThem: String { tr("close-ends-them", "Closing ends them.") }
    public static var terminalInWindowEnds: String { tr("close-window-one", "The terminal in this window will end.") }
    public static func terminalsInWindowEnd(_ n: Int) -> String { tr("close-window-many", "The \(n) terminals in this window will end.") }
    public static var terminalInGroupEnds: String { tr("close-group-one", "The terminal in this group will end.") }
    public static func terminalsInGroupEnd(_ n: Int) -> String { tr("close-group-many", "The \(n) terminals in this group will end.") }

    public static var restartTitle: String { tr("update-restart-title", "Restart to Update?") }
    public static var restart: String { tr("update-restart", "Restart") }
    public static var restartWhenIdle: String { tr("update-restart-when-idle", "Restart When Idle") }
    public static var later: String { tr("update-later", "Later") }
    public static var restartNote: String {
        tr("update-restart-note", "Your terminals come back after the restart, but what runs in them ends.")
    }
}
