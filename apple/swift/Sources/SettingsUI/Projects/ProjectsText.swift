// The Project folders page's own words, named like the Fluent keys of `settings-projects.ftl`. The core writes the rest
// (what a project's trust says, each feature's line) in the language in use.

import WordsModel

enum ProjectsText {
    static var project: String { Words.text("settings-projects-project", fallback: "Project") }
    static var allProjects: String { Words.text("settings-projects-all-projects", fallback: "All projects") }
    static var allProjectsDefaults: String {
        Words.text("settings-projects-all-projects-defaults", fallback: "All projects (defaults)")
    }
    static func missing(_ folder: String) -> String {
        Words.text("settings-projects-missing-folder", fallback: "\(folder) has no .wormhole folder anymore.")
    }
    static var byDefault: String { Words.text("settings-projects-use-from-projects-by-default", fallback: "Use from projects by default") }
    static var defaultsNote: String {
        Words.text(
            "settings-projects-defaults-note", fallback: "A project can still turn a feature on or off just for you: pick it above.")
    }
    static var trustedProjects: String { Words.text("settings-projects-trusted-projects", fallback: "Trusted projects") }
    static var trustedAside: String {
        Words.text("settings-projects-projects-allowed-to-run-commands", fallback: "Projects allowed to run commands")
    }
    static var none: String { Words.text("settings-projects-none-yet", fallback: "None yet.") }
    static var settings: String { Words.text("settings-projects-settings", fallback: "Settings") }
    static var revoke: String { Words.text("settings-projects-revoke", fallback: "Revoke") }
    static var trust: String { Words.text("settings-projects-trust", fallback: "Trust") }
    static var thisProject: String { Words.text("settings-projects-this-project", fallback: "This project") }
    static var revokeTrust: String { Words.text("settings-projects-revoke-trust", fallback: "Revoke trust") }
    static var waiting: String { Words.text("settings-projects-waiting-for-approval", fallback: "Waiting for approval…") }
    static var trustTouchID: String { Words.text("settings-projects-trust-with-touch-id", fallback: "Trust with Touch ID…") }
    static var fromThisProject: String { Words.text("settings-projects-use-from-this-project", fallback: "Use from this project") }
    static var onlyForYou: String {
        Words.text("settings-projects-only-for-you-nothing-is-written", fallback: "Only for you: nothing is written into the project folder.")
    }
    static var defaultOn: String { Words.text("settings-projects-default-on", fallback: "Default (on)") }
    static var defaultOff: String { Words.text("settings-projects-default-off", fallback: "Default (off)") }
    static var on: String { Words.text("settings-projects-on", fallback: "On") }
    static var off: String { Words.text("settings-projects-off", fallback: "Off") }
    static var problems: String { Words.text("settings-projects-problems", fallback: "Problems") }
    static var openTerminal: String { Words.text("settings-projects-open-in-a-new-terminal", fallback: "Open in a new terminal") }
    static func showInFinder(_ folder: String) -> String {
        Words.text("settings-projects-show-in-finder", fallback: "Show \(folder) in Finder")
    }
}
