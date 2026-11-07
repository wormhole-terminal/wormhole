// What the Studio window says, one function per Fluent message of `crates/wormhole-text/locales/en/studio.ftl`,
// named like its key (`studio-show-yours` is `showYours`). English until the FFI hands over the catalogue.

import Foundation
import WordsModel

enum StudioWords {
    private static func text(_ key: StaticString, _ english: String.LocalizationValue) -> String {
        Words.text(key, fallback: String(localized: key, defaultValue: english))
    }

    static let title = text("studio-title", "Theme Studio")
    static let unsaved = text("studio-unsaved", "Unsaved")
    static let save = text("studio-save-theme", "Save theme")
    static let saveShort = text("studio-save", "Save")
    static let export = text("studio-export", "Export…")
    static let exportJSON = text("studio-export-json", "Export JSON…")
    static let copyJSON = text("studio-copy-json", "Copy JSON")
    static let saveCopy = text("studio-menu-duplicate-short", "Save as a copy")
    static let revert = text("studio-menu-revert", "Revert to Saved")
    static let versions = text("studio-menu-versions", "Browse All Versions…")
    static let trash = text("studio-menu-trash", "Move to Trash…")
    static let rename = text("studio-menu-rename", "Rename…")
    static let tooltipMenu = text("studio-tooltip-menu", "Open a theme, or duplicate, rename and revert this one")
    static func undo(_ step: String?) -> String { step.map { text("studio-undo-button", "Undo \($0)") } ?? text("studio-undo-nothing", "Undo") }
    static func redo(_ step: String?) -> String { step.map { text("studio-redo-button", "Redo \($0)") } ?? text("studio-redo-nothing", "Redo") }
    static let light = text("studio-appearance-light", "Light")
    static let dark = text("studio-appearance-dark", "Dark")
    static let appearance = text("studio-appearance-label", "Appearance")
    static func extends(_ base: String, _ appearance: String) -> String { text("studio-title-extends", "extends \(base) · \(appearance)") }
    static func ownPalette(_ appearance: String) -> String { text("studio-title-own", "its own palette · \(appearance)") }
    static let reveal = text("studio-reveal", "Reveal in Finder")

    static let filterPlaceholder = text("studio-filter-placeholder", "Filter tokens")
    static let showAll = text("studio-show-all", "All")
    static func showYours(_ count: Int) -> String { text("studio-show-yours", "Yours · \(count)") }
    static func showLow(_ count: Int) -> String { text("studio-show-low", "Low contrast · \(count)") }
    static let originYours = text("studio-origin-yours", "yours")
    static let originWorkedOut = text("studio-origin-worked-out", "worked out")
    static let tipYours = text("studio-origin-tip-yours", "This theme sets this value itself.")
    static let tipWorkedOut = text("studio-origin-tip-worked-out", "Wormhole works this out from other colours, so it follows your edits. Editing it makes it yours.")
    static func notInArea(_ name: String, _ area: String) -> String { text("studio-not-in-area", "\(name) isn’t used in \(area).") }
    static let reachBoth = text("studio-reach-both", "both")
    static func reachTipBoth(_ appearance: String) -> String { text("studio-reach-tip-both", "Both appearances use this value. Click to set it for \(appearance) only.") }
    static func reachTipThis(_ appearance: String) -> String { text("studio-reach-tip-this", "Only \(appearance) uses this value. Click to set it for both appearances.") }

    static let pickAToken = text("studio-pick-a-token", "Pick a token in the list, or click a color in the gallery, to edit it here.")
    static func followers(_ description: String, _ count: Int) -> String {
        count == 1 ? text("studio-followers-one", "\(description). 1 token follows it.") : text("studio-followers", "\(description). \(count) tokens follow it.")
    }
    static let sectionValue = text("studio-section-value", "Value")
    static let sectionRule = text("studio-section-rule", "Rule")
    static let sectionUsedBy = text("studio-section-used-by", "Used by")
    static let reset = text("studio-reset", "Reset")
    static let tagLastGood = text("studio-tag-last-good", "last good value")
    static let opaque = text("studio-srgb-opaque", "sRGB · opaque")
    static func alpha(_ percent: Int) -> String { text("studio-srgb-alpha", "sRGB · \(percent)% alpha") }
    static let expressionPlaceholder = text("studio-expression-placeholder", "#hex or expression, e.g. primary / 15%")
    static let didYouMean = text("studio-did-you-mean", "Did you mean")
    static func metricFollow(_ source: String, _ value: String) -> String { text("studio-metric-follow", "Follow \(source) (\(value) px)") }
    static let helpWidth = text("studio-metric-help-width", "A number of pixels. No color picker for metrics.")
    static let helpRadius = text("studio-metric-help-radius", "A number of pixels; a radius above half the height is fully round. No color picker for metrics.")
    static func px(_ value: String) -> String { text("studio-metric-px", "\(value) px") }
    static let ruleSetHere = text("studio-rule-set-here", "This theme sets it, so the rule isn't used; Reset goes back to the rule.")
    static let ruleFixed = text("studio-rule-fixed", "A fixed value. Typing a value overrides it; Reset goes back to the rule.")
    static func ruleFollows(_ names: String) -> String { text("studio-rule-follows", "Follows \(names). Typing a value overrides it; Reset goes back to the rule.") }
    static let ruleFromPalette = text("studio-rule-palette-short", "From the terminal's palette. Typing a value overrides it.")
    static let usedByNone = text("studio-used-by-none", "No component draws with it directly; what follows it does.")
    static let usedByOutlined = text("studio-used-by-outlined", "Outlined in the gallery while selected.")
    static let contrast = text("studio-contrast", "Contrast")
    static let contrastOnFill = text("studio-contrast-on-fill", "Contrast on this fill")
    static func contrastOn(_ background: String) -> String { text("studio-contrast-on", "on \(background)") }
    static func contrastMin(_ ratio: String) -> String { text("studio-contrast-min", "min \(ratio)") }
    static func lowCount(_ count: Int) -> String { text("studio-low-contrast-count", "Low contrast · \(count)") }
    static let show = text("studio-contrast-show", "Show")

    static func versionsTitle(_ name: String) -> String { text("studio-versions-title", "Versions of “\(name)”") }
    static func original(_ when: String) -> String { text("studio-versions-original", "\(when) · original file") }
    static let restore = text("studio-versions-restore", "Restore")
    static let done = text("studio-versions-done", "Done")
    static let versionsNote = text("studio-versions-note", "Older saves thin out to one an hour, then one a day. The original file is always kept.")
    static let versionsEmpty = text("studio-versions-empty", "No earlier saves yet. Every save keeps the file it replaces.")
    static let unreadable = text("studio-versions-unreadable", "Can’t be read")
    static let versionsCurrent = text("studio-versions-current", "current")

    static let namePrompt = text("studio-name-prompt", "Name your theme, then press Return.")
    static func closeMessage(_ name: String) -> String { text("studio-close-message", "Do you want to save the changes to “\(name)”?") }
    static let closeDetail = text("studio-unsaved-detail-short", "Your changes are lost if you don’t save them.")
    static let dontSave = text("studio-dont-save", "Don’t Save")
    static let cancel = text("studio-cancel", "Cancel")
    static func changedMessage(_ name: String) -> String { text("studio-changed-message", "“\(name)” was changed by another application.") }
    static let changedDetail = text("studio-changed-detail", "Saving replaces those changes. The file as it is now is kept in the theme’s saved versions.")
    static let overwrite = text("studio-overwrite", "Overwrite")
    static func trashMessage(_ name: String) -> String { text("studio-trash-message", "Move “\(name)” to the Trash?") }
    static let trashDetail = text("studio-trash-detail", "A theme in use falls back to the default theme, and unsaved edits are dropped.")
}
