// What the Studio window says, one function per Fluent message of `crates/wormhole-text/locales/en/studio.ftl`,
// named like its key (`studio-show-yours` is `showYours`). English until the FFI hands over the catalogue.

import Foundation
import WordsModel

enum StudioWords {
    private static func text(_ key: StaticString, _ english: String.LocalizationValue) -> String {
        Words.text(key, fallback: String(localized: key, defaultValue: english))
    }
    private static func text(_ key: StaticString, _ args: [String: WordValue], _ english: String.LocalizationValue) -> String {
        Words.text(key, args, fallback: String(localized: key, defaultValue: english))
    }

    static var title: String { text("studio-title", "Theme Studio") }
    static var unsaved: String { text("studio-unsaved", "Unsaved") }
    static var save: String { text("studio-save-theme", "Save theme") }
    static var saveShort: String { text("studio-save", "Save") }
    static var export: String { text("studio-export", "Export…") }
    static var exportJSON: String { text("studio-export-json", "Export JSON…") }
    static var copyJSON: String { text("studio-copy-json", "Copy JSON") }
    static var revert: String { text("studio-menu-revert", "Revert to Saved") }
    static var versions: String { text("studio-menu-versions", "Browse All Versions…") }
    static var trash: String { text("studio-menu-trash", "Move to Trash…") }
    static var rename: String { text("studio-menu-rename", "Rename…") }
    static var tooltipMenu: String { text("studio-tooltip-menu", "Open a theme, or duplicate, rename and revert this one") }
    static func undo(_ step: String?) -> String { step.map { text("studio-undo-button", ["step": .text($0)], "Undo \($0)") } ?? text("studio-undo-nothing", "Undo") }
    static func redo(_ step: String?) -> String { step.map { text("studio-redo-button", ["step": .text($0)], "Redo \($0)") } ?? text("studio-redo-nothing", "Redo") }
    static var light: String { text("studio-appearance-light", "Light") }
    static var dark: String { text("studio-appearance-dark", "Dark") }
    static var appearance: String { text("studio-appearance-label", "Appearance") }
    static func extends(_ base: String, _ appearance: String) -> String { text("studio-title-extends", ["base": .text(base), "appearance": .text(appearance)], "extends \(base) · \(appearance)") }
    static func ownPalette(_ appearance: String) -> String { text("studio-title-own", ["appearance": .text(appearance)], "its own palette · \(appearance)") }
    static var reveal: String { text("studio-reveal", "Reveal in Finder") }

    static var filterPlaceholder: String { text("studio-filter-placeholder", "Filter tokens") }
    static var showAll: String { text("studio-show-all", "All") }
    static func showYours(_ count: Int) -> String { text("studio-show-yours", ["count": .whole(count)], "Yours · \(count)") }
    static func showLow(_ count: Int) -> String { text("studio-show-low", ["count": .whole(count)], "Low contrast · \(count)") }
    static var originYours: String { text("studio-origin-yours", "yours") }
    static var originWorkedOut: String { text("studio-origin-worked-out", "worked out") }
    static var tipYours: String { text("studio-origin-tip-yours", "This theme sets this value itself.") }
    static var tipWorkedOut: String { text("studio-origin-tip-worked-out", "Wormhole works this out from other colours, so it follows your edits. Editing it makes it yours.") }
    static func notInArea(_ name: String, _ area: String) -> String { text("studio-not-in-area", ["name": .text(name), "area": .text(area)], "\(name) isn’t used in \(area).") }
    static var reachBoth: String { text("studio-reach-both", "both") }
    static func reachTipBoth(_ appearance: String) -> String { text("studio-reach-tip-both", ["appearance": .text(appearance)], "Both appearances use this value. Click to set it for \(appearance) only.") }
    static func reachTipThis(_ appearance: String) -> String { text("studio-reach-tip-this", ["appearance": .text(appearance)], "Only \(appearance) uses this value. Click to set it for both appearances.") }

    static var pickAToken: String { text("studio-pick-a-token", "Pick a token in the list, or click a color in the gallery, to edit it here.") }
    static func followers(_ description: String, _ count: Int) -> String {
        text("studio-followers", ["description": .text(description), "count": .whole(count)], count == 1 ? "\(description). 1 token follows it." : "\(description). \(count) tokens follow it.")
    }
    static var sectionValue: String { text("studio-section-value", "Value") }
    static var sectionRule: String { text("studio-section-rule", "Rule") }
    static var sectionUsedBy: String { text("studio-section-used-by", "Used by") }
    static var reset: String { text("studio-reset", "Reset") }
    static var tagLastGood: String { text("studio-tag-last-good", "last good value") }
    static var opaque: String { text("studio-srgb-opaque", "sRGB · opaque") }
    static func alpha(_ percent: Int) -> String { text("studio-srgb-alpha", ["percent": .whole(percent)], "sRGB · \(percent)% alpha") }
    static var expressionPlaceholder: String { text("studio-expression-placeholder", "#hex or expression, e.g. primary / 15%") }
    static var didYouMean: String { text("studio-did-you-mean", "Did you mean") }
    static func metricFollow(_ source: String, _ value: String) -> String { text("studio-metric-follow", ["source": .text(source), "value": .text(value)], "Follow \(source) (\(value) px)") }
    static var helpWidth: String { text("studio-metric-help-width", "A number of pixels. No color picker for metrics.") }
    static var helpRadius: String { text("studio-metric-help-radius", "A number of pixels; a radius above half the height is fully round. No color picker for metrics.") }
    static func px(_ value: String) -> String { text("studio-metric-px", ["value": .text(value)], "\(value) px") }
    static var ruleSetHere: String { text("studio-rule-set-here", "This theme sets it, so the rule isn't used; Reset goes back to the rule.") }
    static var ruleFixed: String { text("studio-rule-fixed", "A fixed value. Typing a value overrides it; Reset goes back to the rule.") }
    static func ruleFollows(_ names: String) -> String { text("studio-rule-follows", ["names": .text(names)], "Follows \(names). Typing a value overrides it; Reset goes back to the rule.") }
    static var ruleFromPalette: String { text("studio-rule-palette-short", "From the terminal's palette. Typing a value overrides it.") }
    static var usedByNone: String { text("studio-used-by-none", "No component draws with it directly; what follows it does.") }
    static var usedByOutlined: String { text("studio-used-by-outlined", "Outlined in the gallery while selected.") }
    static var contrast: String { text("studio-contrast", "Contrast") }
    static var contrastOnFill: String { text("studio-contrast-on-fill", "Contrast on this fill") }
    static func contrastOn(_ background: String) -> String { text("studio-contrast-on", ["background": .text(background)], "on \(background)") }
    static func contrastMin(_ ratio: String) -> String { text("studio-contrast-min", ["ratio": .text(ratio)], "min \(ratio)") }
    static func lowCount(_ count: Int) -> String { text("studio-low-contrast-count", ["count": .whole(count)], "Low contrast · \(count)") }
    static var show: String { text("studio-contrast-show", "Show") }

    static func versionsTitle(_ name: String) -> String { text("studio-versions-title", ["name": .text(name)], "Versions of “\(name)”") }
    static func original(_ when: String) -> String { text("studio-versions-original", ["when": .text(when)], "\(when) · original file") }
    static var restore: String { text("studio-versions-restore", "Restore") }
    static var done: String { text("studio-versions-done", "Done") }
    static var versionsNote: String { text("studio-versions-note", "Older saves thin out to one an hour, then one a day. The original file is always kept.") }
    static var versionsEmpty: String { text("studio-versions-empty", "No earlier saves yet. Every save keeps the file it replaces.") }
    static var unreadable: String { text("studio-versions-unreadable", "Can’t be read") }
    static var versionsCurrent: String { text("studio-versions-current", "current") }

    static var namePrompt: String { text("studio-name-prompt", "Name your theme, then press Return.") }
    static func closeMessage(_ name: String) -> String { text("studio-close-message", ["name": .text(name)], "Do you want to save the changes to “\(name)”?") }
    static func closeDetail(_ changes: Int) -> String {
        text("studio-unsaved-detail", ["count": .whole(changes)], changes == 1 ? "1 change. Your changes are lost if you don’t save them." : "\(changes) changes. Your changes are lost if you don’t save them.")
    }
    static var dontSave: String { text("studio-dont-save", "Don’t Save") }
    static var cancel: String { text("studio-cancel", "Cancel") }
    static func changedMessage(_ name: String) -> String { text("studio-changed-message", ["name": .text(name)], "“\(name)” was changed by another application.") }
    static var changedDetail: String { text("studio-changed-detail", "Saving replaces those changes. The file as it is now is kept in the theme’s saved versions.") }
    static var overwrite: String { text("studio-overwrite", "Overwrite") }
    static func trashMessage(_ name: String) -> String { text("settings-appearance-trash-message", ["name": .text(name)], "Move “\(name)” to the Trash?") }
    static var trashDetail: String { text("studio-trash-detail", "A theme in use falls back to the default theme, and unsaved edits are dropped.") }

    static var yourThemes: String { text("studio-menu-your-themes", "Your Themes") }
    static var wormhole: String { text("studio-menu-wormhole", "Wormhole") }
    static var saveMore: String { text("studio-save-more", "More ways to save") }
    static var nameField: String { text("studio-name-field", "Theme name") }
    static var tokenList: String { text("studio-list-label", "Tokens") }
    static var clearFilter: String { text("studio-filter-clear", "Clear the filter") }
    static var tokenEditor: String { text("studio-editor-label", "Token editor") }
    static var bandExpanded: String { text("studio-band-expanded", "expanded") }
    static var bandCollapsed: String { text("studio-band-collapsed", "collapsed") }
    static func bandLow(_ count: Int) -> String {
        text("studio-band-low-hint", ["count": .whole(count)], count == 1 ? "1 falls short of a contrast minimum" : "\(count) fall short of a contrast minimum")
    }
    static var rowLow: String { text("studio-row-low-hint", "falls short of a contrast minimum") }
    static var passes: String { text("studio-contrast-passes", "passes") }
    static var fallsShort: String { text("studio-contrast-falls-short", "falls short") }
    static func ratioSpoken(_ ratio: String, _ minimum: String) -> String {
        text("studio-contrast-ratio-spoken", ["ratio": .text(ratio), "min": .text(minimum)], "\(ratio) to 1, minimum \(minimum)")
    }
    static var galleryLabel: String { text("studio-gallery-label", "Gallery") }
    static var galleryAreas: String { text("studio-gallery-areas", "Show the part of the app") }
    static var gallerySelected: String { text("studio-gallery-uses-selected", "uses the selected token") }
    static var showPalette: String { text("studio-gallery-show-palette", "Show the palette") }
    static var showSheet: String { text("studio-gallery-show-sheet", "Show the shortcut sheet") }
    static var showQuickPanel: String { text("studio-gallery-show-quick-panel", "Show the quick panel") }
    static var turnCount: String { text("studio-gallery-turn-count", "Turn the counts over") }
    static var systemTooltip: String { text("studio-gallery-system-tooltip", "macOS draws tooltips; a theme does not change them.") }
    static var tooltipHere: String { text("studio-gallery-tooltip-here", "Rest the pointer here") }
    static var tooltipSample: String { text("studio-gallery-tooltip-sample", "A tooltip explains a control") }
    static var systemMenu: String { text("studio-gallery-system-menu", "macOS draws menus; a theme does not change them.") }
    static var showMenu: String { text("studio-gallery-show-menu", "Open a menu") }
    static var systemDialog: String { text("studio-gallery-system-dialog", "macOS draws questions as its own alerts; a theme does not change them.") }
    static var showDialog: String { text("studio-gallery-show-dialog", "Show a question") }
    static var systemScrollbar: String { text("studio-gallery-system-scrollbar", "macOS draws the scroll bars; a theme does not change them.") }
    static var systemAbout: String { text("studio-gallery-system-about", "About Wormhole is the system\u{2019}s own panel; a theme does not change it.") }
    static var showAbout: String { text("studio-gallery-show-about", "Show About Wormhole") }

    // The theme menu, the title row's marks and the update sheet.
    static var menuThisProject: String { text("studio-menu-this-project", "This Project") }
    static var menuProjectTheme: String { text("studio-menu-project-theme", "The project’s theme") }
    static var menuNew: String { text("studio-menu-new", "New Theme…") }
    static var menuNewFrom: String { text("studio-menu-new-from", "New Theme From") }
    static var menuAddDark: String { text("studio-menu-add-dark", "Add Dark Appearance") }
    static var menuAddLight: String { text("studio-menu-add-light", "Add Light Appearance") }
    static var menuRevertTo: String { text("studio-menu-revert-to", "Revert To") }
    static func menuDuplicate(_ name: String) -> String { text("studio-menu-duplicate", ["name": .text(name)], "Duplicate “\(name)”") }
    static func updateFrom(_ from: String) -> String { text("studio-update-from", ["from": .text(from)], "Update from \(from)…") }
    static func updateCurrent(_ from: String) -> String { text("studio-menu-update-current", ["from": .text(from)], "Up to date with \(from)") }
    static func updateMissing(_ from: String) -> String { text("studio-menu-update-missing", ["from": .text(from)], "\(from) is unavailable") }
    static var useInProject: String { text("studio-use-in-project", "Use in this project…") }
    static var useWithMatchSystem: String { text("studio-use-with-match-system", "Use with Match system") }
    static var newerChip: String { text("studio-newer-chip", "Made with a newer Wormhole") }
    static var newerDuplicate: String { text("studio-newer-duplicate", "Duplicate") }
    static func newerExplain(_ count: Int) -> String {
        text("studio-newer-explain", ["count": .whole(count)], count == 1 ? "1 value this version doesn’t know was skipped. It shows what it can; the rest is worked out. Saving would drop them, so Theme Studio won’t save over it." : "\(count) values this version doesn’t know were skipped. It shows what it can; the rest is worked out. Saving would drop them, so Theme Studio won’t save over it.")
    }
    static var previewChip: String { text("studio-preview-chip", "Preview") }
    static var previewUse: String { text("studio-preview-use", "Use") }
    static var previewTooltip: String { text("studio-preview-tooltip", "Every window shows this theme while Theme Studio is open. Closing Theme Studio puts your theme back.") }
    static func titleBasedOn(_ from: String, _ day: String, _ appearance: String) -> String {
        text("studio-title-based-on", ["from": .text(from), "day": .text(day), "appearance": .text(appearance)], "based on \(from) · copied \(day) · \(appearance)")
    }
    static func titleGenerated(_ from: String, _ appearance: String) -> String { text("studio-title-generated", ["from": .text(from), "appearance": .text(appearance)], "generated from \(from) · \(appearance)") }
    static func showBase(_ count: Int) -> String { text("studio-show-base", ["count": .whole(count)], "Base · \(count)") }
    static var originBase: String { text("studio-origin-base", "base") }
    static func tipBase(_ from: String) -> String { text("studio-origin-tip-base", ["from": .text(from)], "Copied from \(from) and not changed. Editing it makes it yours.") }
    static var resetToBase: String { text("studio-reset-base", "Reset to base") }
    static func updateTitle(_ from: String) -> String { text("studio-update-title", ["from": .text(from)], "Update from \(from)?") }
    static func updateCount(_ count: Int, _ date: String) -> String {
        text("studio-update-count", ["count": .whole(count), "date": .text(date)], count == 1 ? "1 value changed since you copied it on \(date). Your own changes stay." : "\(count) values changed since you copied it on \(date). Your own changes stay.")
    }
    static var updateKept: String { text("studio-update-kept", "yours · kept") }
    static var updateApply: String { text("studio-update-apply", "Update") }
    static func existsMessage(_ name: String) -> String { text("studio-exists-message", ["name": .text(name)], "“\(name)” already exists. Do you want to replace it?") }
    static var existsDetail: String { text("studio-exists-detail", "A theme with the same name is in your themes folder. Replacing it overwrites that theme with this draft.") }
    static var replace: String { text("studio-replace", "Replace") }
    static func switchMessage(_ current: String, _ next: String) -> String {
        text("studio-switch-message", ["current": .text(current), "next": .text(next)], "Do you want to save the changes to “\(current)” before opening “\(next)”?")
    }
    static func copyFailed(_ reason: String) -> String { text("studio-copy-failed", ["error": .text(reason)], "Could not save a copy: \(reason)") }
    static var openFailed: String { text("studio-open-failed", "That theme could not be opened.") }
}
