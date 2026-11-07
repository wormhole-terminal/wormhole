// The window's own words, one function each, named like the Fluent keys of `settings-window.ftl`
// (`settings-window-search-settings`). They go through `String(localized:)` with the English as the default, so a
// string table (or the FFI's text table, when it exists) replaces them without touching a view. The schema's
// words (labels, descriptions, options) arrive already in the language in use.

import Foundation
import WordsModel

enum SettingsText {
    static var settings: String { Words.text("settings-window-settings", fallback: "Settings") }
    static var searchSettings: String {
        Words.text("settings-window-search-settings", fallback: "Search settings")
    }
    static var clearSearch: String { Words.text("settings-window-clear-search", fallback: "Clear search") }
    static var results: String { Words.text("settings-window-results", fallback: "Results") }
    static var noResults: String {
        String(
            localized: "settings-window-no-results",
            defaultValue: "No setting has these words in its name, description or config key.")
    }
    static var escClears: String {
        Words.text("settings-window-clears-the-search-then-closes", fallback: "clears the search, then closes")
    }
    static func resultsSummary(hits: Int, sections: Int, query: String) -> String {
        String(
            localized: "settings-window-results-summary",
            defaultValue: "\(hits) settings in \(sections) sections match “\(query)”. Pick a section to see it in context.")
    }
    static var rescanPath: String { Words.text("settings-window-rescan-path", fallback: "Rescan PATH") }
    static var scanAgain: String { Words.text("settings-import-scan-again", fallback: "Scan again") }
    static var newProfile: String { Words.text("settings-window-new-profile", fallback: "New profile…") }
    static var notSet: String { Words.text("settings-row-not-set", fallback: "Not set") }
    static var noneSet: String { Words.text("settings-row-none", fallback: "None") }
    static var notBuilt: String { Words.text("settings-panel-not-built", fallback: "Not in this build yet") }
    static var install: String { Words.text("settings-agents-install", fallback: "Install") }
    static var update: String { Words.text("settings-agents-update", fallback: "Update") }
    static var uninstall: String { Words.text("settings-agents-uninstall", fallback: "Uninstall") }
    static var copyLine: String { Words.text("settings-agents-copy-line", fallback: "Copy line") }
    static var checkingPath: String {
        Words.text("settings-agents-checking-your-shells-path", fallback: "Checking your shell's PATH…")
    }
    static var accessOff: String { Words.text("settings-agents-access-off", fallback: "Off") }
    static var accessAsk: String { Words.text("settings-agents-access-ask", fallback: "Ask") }
    static var accessOpen: String { Words.text("settings-agents-access-open", fallback: "Allowed while open") }
    static var revoke: String { Words.text("settings-agents-access-revoke", fallback: "Revoke") }
    static var noApprovals: String { Words.text("settings-agents-access-none", fallback: "Nothing has asked yet.") }
    static var approvalsNote: String {
        Words.text(
            "settings-agents-access-caps-note",
            fallback: "Closing a terminal always asks. A program uses the terminals it opened and the one it runs in; for any other it asks you first.")
    }
    static var deleteProfile: String { Words.text("settings-profile-delete-profile", fallback: "Delete profile…") }
    static var deleteProfileTitle: String { Words.text("settings-profile-delete", fallback: "Delete profile") }
    static var cancel: String { Words.text("settings-profile-cancel", fallback: "Cancel") }
    static var delete: String { Words.text("settings-profile-delete-button", fallback: "Delete") }
    static var newProfileTitle: String { Words.text("settings-profile-new-profile", fallback: "New profile") }
    static var profileName: String { Words.text("settings-profile-name-field", fallback: "Name") }
    static var startFrom: String { Words.text("settings-profile-start-from", fallback: "Start from") }
    static var wormholeDefaults: String {
        Words.text("settings-profile-wormholes-defaults", fallback: "Wormhole’s defaults")
    }
    static var giveAName: String { Words.text("settings-profile-give-the-profile-a-name", fallback: "Give the profile a name") }
    static var loginsNeverCopied: String {
        Words.text(
            "settings-profile-logins-never-copied",
            fallback: "Agent logins are never copied: the new profile signs in on its own.")
    }
    static var create: String { Words.text("settings-profile-create", fallback: "Create") }
    static var preview: String { Words.text("settings-window-preview", fallback: "Preview") }
    static var previewTag: String { Words.text("settings-nav-preview-tag", fallback: "Preview") }
    static var reset: String { Words.text("settings-row-reset", fallback: "Reset to default") }
    static var changed: String { Words.text("settings-row-changed", fallback: "Changed") }
    static var changedTitle: String { Words.text("settings-section-changed", fallback: "Changed") }
    static var changedIntro: String {
        String(
            localized: "settings-section-changed-intro",
            defaultValue: "Every setting that differs from its default, each put back by its arrow.")
    }
    static var noneChanged: String { Words.text("settings-changed-none", fallback: "No settings changed") }
    static var noneChangedDetail: String {
        Words.text("settings-changed-none-detail", fallback: "Everything is at its default")
    }
    static var resetAll: String { Words.text("settings-changed-reset-all", fallback: "Reset All…") }
    static var recorderPrompt: String {
        Words.text("settings-row-recorder-prompt", fallback: "Press the shortcut…")
    }
    static var recorderNone: String { Words.text("settings-row-recorder-none", fallback: "Not set") }
    static var profileDefault: String { Words.text("settings-nav-profile-default", fallback: "Default") }
    static func previewLabel(_ scene: String) -> String {
        Words.text("settings-window-preview-label", fallback: "Preview of \(scene) with your current settings")
    }
}
