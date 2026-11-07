// The window's own words, one function each, named like the Fluent keys of `settings-window.ftl`
// (`settings-window-search-settings`). They go through `String(localized:)` with the English as the default, so a
// string table (or the FFI's text table, when it exists) replaces them without touching a view. The schema's
// words (labels, descriptions, options) arrive already in the language in use.

import Foundation

enum SettingsText {
    static var settings: String { String(localized: "settings-window-settings", defaultValue: "Settings") }
    static var searchSettings: String {
        String(localized: "settings-window-search-settings", defaultValue: "Search settings")
    }
    static var clearSearch: String { String(localized: "settings-window-clear-search", defaultValue: "Clear search") }
    static var results: String { String(localized: "settings-window-results", defaultValue: "Results") }
    static var noResults: String {
        String(
            localized: "settings-window-no-results",
            defaultValue: "No setting has these words in its name, description or config key.")
    }
    static var escClears: String {
        String(localized: "settings-window-clears-the-search-then-closes", defaultValue: "clears the search, then closes")
    }
    static func resultsSummary(hits: Int, sections: Int, query: String) -> String {
        String(
            localized: "settings-window-results-summary",
            defaultValue: "\(hits) settings in \(sections) sections match “\(query)”. Pick a section to see it in context.")
    }
    static var rescanPath: String { String(localized: "settings-window-rescan-path", defaultValue: "Rescan PATH") }
    static var scanAgain: String { String(localized: "settings-import-scan-again", defaultValue: "Scan again") }
    static var newProfile: String { String(localized: "settings-window-new-profile", defaultValue: "New profile…") }
    static var notSet: String { String(localized: "settings-row-not-set", defaultValue: "Not set") }
    static var noneSet: String { String(localized: "settings-row-none", defaultValue: "None") }
    static var notBuilt: String { String(localized: "settings-panel-not-built", defaultValue: "Not in this build yet") }
    static var preview: String { String(localized: "settings-window-preview", defaultValue: "Preview") }
    static var previewTag: String { String(localized: "settings-nav-preview-tag", defaultValue: "Preview") }
    static var reset: String { String(localized: "settings-row-reset", defaultValue: "Reset to default") }
    static var changed: String { String(localized: "settings-row-changed", defaultValue: "Changed") }
    static var changedTitle: String { String(localized: "settings-section-changed", defaultValue: "Changed") }
    static var changedIntro: String {
        String(
            localized: "settings-section-changed-intro",
            defaultValue: "Every setting that differs from its default, each put back by its arrow.")
    }
    static var noneChanged: String { String(localized: "settings-changed-none", defaultValue: "No settings changed") }
    static var noneChangedDetail: String {
        String(localized: "settings-changed-none-detail", defaultValue: "Everything is at its default")
    }
    static var resetAll: String { String(localized: "settings-changed-reset-all", defaultValue: "Reset All…") }
    static var recorderPrompt: String {
        String(localized: "settings-row-recorder-prompt", defaultValue: "Press the shortcut…")
    }
    static var recorderNone: String { String(localized: "settings-row-recorder-none", defaultValue: "Not set") }
    static var profileDefault: String { String(localized: "settings-nav-profile-default", defaultValue: "Default") }
    static func previewLabel(_ scene: String) -> String {
        String(localized: "settings-window-preview-label", defaultValue: "Preview of \(scene) with your current settings")
    }
}
