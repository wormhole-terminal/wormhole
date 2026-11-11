// The Extensions page's own words, named like the Fluent keys of `settings-extensions.ftl`. What the core writes
// (states, notes, permissions, errors) arrives in the language in use; these are the page's labels.

import WordsModel

enum ExtensionsText {
    static var refresh: String { Words.text("settings-extensions-refresh", fallback: "Scan the Probes folder again") }
    static var looking: String { Words.text("settings-extensions-looking", fallback: "Looking for Probes…") }
    static var noneTitle: String { Words.text("settings-extensions-none-title", fallback: "No Probes yet") }
    static func noneWhere(_ folder: String) -> String {
        Words.text(
            "settings-extensions-none-where", ["folder": .text(folder)],
            fallback: "Probes live in \(folder), one folder each with a probe.toml in it.")
    }
    static func noneNew(_ command: String) -> String {
        Words.text(
            "settings-extensions-none-new", ["command": .text(command)],
            fallback: "Make one with \(command), or copy a folder in. It stays off until you approve it here.")
    }
    static var showFolder: String { Words.text("settings-extensions-show-folder", fallback: "Show in Finder") }
    static var approveAgain: String { Words.text("settings-extensions-approve-again", fallback: "Approve again…") }
    static var forget: String { Words.text("settings-extensions-forget", fallback: "Forget approval") }
    static var touchID: String { Words.text("settings-extensions-touch-id", fallback: "Touch ID") }
    static var since: String { Words.text("settings-extensions-since", fallback: "Since you approved it") }
    static var combinations: String { Words.text("settings-extensions-combinations", fallback: "Dangerous together") }
    static var confirmTouchID: String {
        Words.text("settings-extensions-confirm-touch-id", fallback: "Approving needs Touch ID, or your password.")
    }
    static var confirmClick: String { Words.text("settings-extensions-confirm-click", fallback: "Approving needs only a click.") }
    static var cancel: String { Words.text("settings-extensions-cancel", fallback: "Cancel") }
    static var approve: String { Words.text("settings-extensions-approve", fallback: "Approve") }
    static var approveTouchID: String { Words.text("settings-extensions-approve-touch-id", fallback: "Approve with Touch ID…") }
    static var waiting: String { Words.text("settings-extensions-waiting", fallback: "Waiting for approval…") }
}
