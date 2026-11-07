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
