// The words of the secondary windows. The GPUI app keeps its words in `wormhole-text`'s Fluent catalogs; until the
// FFI delivers that table, each message here is one function whose key is the Fluent key, with the English
// source as its default, so a `.strings` table can replace the defaults without touching a caller.

import Foundation
import WordsModel

public enum AuxText {
    static func tr(_ key: StaticString, _ value: String.LocalizationValue) -> String {
        Words.text(key, fallback: String(localized: key, defaultValue: value, table: "Aux"))
    }

    /// A message with variables: `args` are named as in its Fluent text, `english` has the values in it.
    static func tr(_ key: StaticString, _ args: [String: WordValue], fallback english: String) -> String {
        Words.text(key, args, fallback: english)
    }

    /// The product's name in the catalogs' `-brand` term.
    public static let brand = "Wormhole" // text: the product's name, the catalogs' -brand term
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
        tr("dialog-quit-keeps-all", "Your terminals come back next time and what runs in them keeps going. Untick the box to end them.")
    }

    public static func quitKeepsSome(kept: Int, ended: Int) -> String {
        tr(
            "dialog-quit-keeps-some", ["kept": .whole(kept), "ended": .whole(ended)],
            fallback: "Your terminals come back next time. \(kept == 1 ? "1 terminal keeps" : "\(kept) terminals keep") running; \(ended == 1 ? "1 terminal ends because it runs" : "\(ended) terminals end because they run") inside \(brand). Untick the box to end the others too.")
    }

    public static func quitOtherProfile(count: Int, profile: String) -> String {
        tr(
            "dialog-quit-other-profile", ["count": .whole(count), "profile": .text(profile)],
            fallback: count == 1 ? "1 terminal in \(profile) is running an agent or a command." : "\(count) terminals in \(profile) are running an agent or a command.")
    }

    public static var agentWaiting: String { tr("close-agent-waiting", "An agent is waiting for an answer.") }
    public static var agentWorking: String { tr("close-agent-working", "An agent is still working.") }
    public static var commandRunning: String { tr("close-command-running", "A command is still running.") }
    public static func stillRunning(_ list: String) -> String { tr("close-still-running", ["list": .text(list)], fallback: "Still running: \(list).") }
    public static func isStillWorking(_ who: String) -> String { tr("close-is-working", ["who": .text(who)], fallback: "\(who) is still working.") }
    public static func isWaiting(_ who: String) -> String { tr("close-is-waiting", ["who": .text(who)], fallback: "\(who) is waiting for an answer.") }
    public static func closeSessionTitle(_ name: String) -> String { tr("close-session-title", ["name": .text(name)], fallback: "Close \u{201C}\(name)\u{201D}?") }
    public static func closeGroupTitle(_ name: String) -> String { tr("close-group-title", ["name": .text(name)], fallback: "Close the group \u{201C}\(name)\u{201D}?") }
    public static var itsAgent: String { tr("close-its-agent", "Its agent") }
    public static func itsStake(_ stake: String) -> String { tr("close-its-stake", ["stake": .text(stake)], fallback: "Its \(stake)") }
    public static func theirStake(_ stake: String) -> String { tr("close-their-stake", ["stake": .text(stake)], fallback: "Their \(stake)") }
    public static func andMore(_ count: Int) -> String { tr("close-and-more", ["count": .whole(count)], fallback: "and \(count) more") }
    public static var shellEnds: String { tr("close-shell-ends", "Its shell ends.") }
    /// What the close question adds when a script asked for it (`wh close`), so the person knows why the sheet appeared.
    public static var controlCloseAsked: String { tr("control-close-asked", "The command line asked to close it.") }
    public static var closingEndsProcess: String { tr("close-ends-process", "Closing ends the process.") }
    public static var closingEndsThem: String { tr("close-ends-them", "Closing ends them.") }
    public static var terminalInWindowEnds: String { tr("close-window-one", "The terminal in this window will end.") }
    public static func terminalsInWindowEnd(_ n: Int) -> String { tr("close-window-many", ["count": .whole(n)], fallback: "The \(n) terminals in this window will end.") }
    public static var terminalInGroupEnds: String { tr("close-group-one", "The terminal in this group will end.") }
    public static func terminalsInGroupEnd(_ n: Int) -> String { tr("close-group-many", ["count": .whole(n)], fallback: "The \(n) terminals in this group will end.") }

    public static var restartTitle: String { tr("update-restart-title", "Restart to Update?") }
    public static var restart: String { tr("update-restart", "Restart") }
    public static var restartWhenIdle: String { tr("update-restart-when-idle", "Restart When Idle") }
    public static var later: String { tr("update-later", "Later") }
    public static var restartNote: String {
        tr("update-restart-note", "Your terminals come back after the restart, but what runs in them ends.")
    }
    public static func restartAgentWorking(_ agent: String) -> String { tr("restart-agent-working", ["agent": .text(agent)], fallback: "\(agent) is working") }
    public static func restartAgentWaiting(_ agent: String) -> String { tr("restart-agent-waiting", ["agent": .text(agent)], fallback: "\(agent) is waiting for an answer") }
    public static var restartStillWorking: String { tr("restart-still-working", "still working") }
    public static var restartWaitingForYou: String { tr("restart-waiting-for-you", "waiting for you") }
    public static var restartCommandRunning: String { tr("restart-command-running", "a command is running") }
}
