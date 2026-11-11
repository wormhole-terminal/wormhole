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
    static var saveCopy: String { text("studio-menu-duplicate-short", "Save as a copy") }
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
}
