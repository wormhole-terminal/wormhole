// The words the model speaks (notices, undo steps, theme names), one function per Fluent message of
// `crates/wormhole-text/locales/en/studio.ftl`, named like its key. English until the FFI hands over the
// catalogue's table; `String(localized:)` keeps each one translatable meanwhile.

import Foundation
import WordsModel

public enum StudioNames {
    /// What a new theme is called until it is named, and what a copy of another is.
    public static var untitled: String { StudioText.untitled }
    public static func untitled(from base: String) -> String { StudioText.untitledFrom(base) }
}

enum StudioText {
    private static func text(_ key: StaticString, _ english: String.LocalizationValue) -> String {
        Words.text(key, fallback: String(localized: key, defaultValue: english))
    }
    private static func text(_ key: StaticString, _ args: [String: WordValue], _ english: String.LocalizationValue) -> String {
        Words.text(key, args, fallback: String(localized: key, defaultValue: english))
    }

    static func untitledFrom(_ from: String) -> String { text("studio-untitled-from", ["from": .text(from)], "Untitled (from \(from))") }
    static func customName(_ base: String) -> String { text("studio-custom-name", ["base": .text(base)], "\(base) Custom") }
    static func copyName(_ name: String, n: Int? = nil) -> String {
        n.map { text("studio-copy-name-n", ["name": .text(name), "n": .whole($0)], "\(name) copy \($0)") } ?? text("studio-copy-name", ["name": .text(name)], "\(name) copy")
    }
    /// A pair's version as people read it: its two appearances by their words, any other version as it is named.
    static func appearanceWord(_ name: String) -> String {
        switch name {
        case "light": text("studio-appearance-light", "Light")
        case "dark": text("studio-appearance-dark", "Dark")
        default: name
        }
    }
    static var untitled: String { text("studio-untitled", "Untitled") }
    static var newerFile: String { text("studio-newer-file", "This theme was made by a newer Wormhole. Saving here would lose what this version doesn’t understand, so it was not changed. Duplicate keeps a copy.") }
    static var convertFailed: String { text("studio-convert-failed", "Could not convert this theme safely, so it was not changed.") }
    static var savedConverted: String { text("studio-saved-converted", "Saved in the new theme format: older releases of Wormhole can’t open this file. The previous file is kept in the theme’s saved versions.") }
    static var savedComments: String { text("studio-saved-comments", "Comments in the file were not kept.") }
    static var addFailed: String { text("studio-add-failed", "Could not add an appearance to this theme.") }
    static var openFailed: String { text("studio-open-failed", "That theme could not be opened.") }
    static var undoAddAppearance: String { text("studio-undo-add-appearance", "Add Appearance") }
    static var undoUpdateFrom: String { text("studio-undo-update-from", "Update from Base") }
    static var noTerminal: String { text("studio-no-terminal", "Open a terminal in a project first.") }
    static var noProjectFolder: String { text("studio-no-project-folder", "The terminal in front isn't in a project with a .wormhole folder.") }
    static var otherProject: String { text("studio-other-project", "The terminal in front is in another project.") }
    static func projectSaved(_ path: String) -> String { text("studio-project-saved", ["path": .text(path)], "Saved to \(path): terminals in this project use it.") }
    static func projectFailed(_ reason: String) -> String { text("studio-project-failed", ["error": .text(reason)], "Could not save to the project: \(reason)") }
    static func onlyShown(_ appearance: String) -> String { text("studio-only-shown", ["appearance": .text(appearance)], "Only the \(appearance) appearance of this theme was written.") }
    static func pairUsed(_ name: String) -> String { text("studio-pair-used", ["name": .text(name)], "“\(name)” follows the system’s light and dark.") }
    static var nameFirst: String { text("studio-name-first", "Give the theme a name first.") }
    static func saved(_ path: String) -> String { text("studio-saved", ["path": .text(path)], "Saved to \(path).") }
    static func savedAndSet(_ path: String) -> String { text("studio-saved-set", ["path": .text(path)], "Saved to \(path) and set as your theme.") }
    static func copySaved(_ name: String, _ path: String) -> String { text("studio-copy-saved", ["name": .text(name), "path": .text(path)], "Saved a copy as “\(name)” to \(path).") }
    static func saveFailed(_ error: String) -> String { text("studio-save-failed", ["error": .text(error)], "Could not save: \(error)") }
    static func exportFailed(_ error: String) -> String { text("studio-export-failed", ["error": .text(error)], "Could not export: \(error)") }
    static func exported(_ path: String) -> String { text("studio-exported", ["path": .text(path)], "Exported to \(path).") }
    static var copiedJSON: String { text("studio-copied-json", "Copied the theme's JSON.") }
    static func nameTaken(_ name: String) -> String {
        text("studio-name-taken", ["name": .text(name)], "Another theme is already called “\(name)”. Choose another name.")
    }
    static func nameIsVersion(_ name: String) -> String {
        text("studio-name-is-a-version", ["name": .text(name)], "“\(name)” is the name of one version of a theme that comes in several. Choose another name.")
    }
    static func trashFailed(_ name: String) -> String { text("settings-appearance-trash-failed", ["name": .text(name)], "“\(name)” could not be moved to the Trash.") }
    static func undoChange(_ token: String) -> String { text("studio-undo-change", ["token": .text(token)], "Change “\(token)”") }
    static func undoReset(_ token: String) -> String { text("studio-undo-reset", ["token": .text(token)], "Reset “\(token)”") }
    static var undoRename: String { text("studio-undo-rename", "Rename") }
    static var undoRevert: String { text("studio-undo-revert", "Revert to Saved") }
    static var undoRestore: String { text("studio-undo-restore", "Restore Version") }
    static func undoMove(_ token: String) -> String { text("studio-undo-move", ["token": .text(token)], "Move “\(token)”") }
    static var versionsFirst: String { text("studio-versions-first", "first save") }
    static var versionsCurrent: String { text("studio-versions-current", "current") }
    static var versionsJustNow: String { text("studio-versions-just-now", "just now") }
    static func notPixels(_ name: String, _ value: String) -> String {
        text("studio-not-pixels", ["name": .text(name), "value": .text(value)], "\(name): “\(value)” is not a number of pixels")
    }
    static func lighten(_ ratio: String) -> String { text("studio-contrast-fix-lighten", ["ratio": .text(ratio)], "Lighten to \(ratio):1") }
    static func darken(_ ratio: String) -> String { text("studio-contrast-fix-darken", ["ratio": .text(ratio)], "Darken to \(ratio):1") }
    static func useReadable(_ current: String, _ ratio: String) -> String {
        text("studio-contrast-fix-readable", ["current": .text(current), "ratio": .text(ratio)], "Use readable(\(current), …, \(ratio))")
    }
    static var defaultName: String { text("studio-default-name", "Theme") }
    static var folderTooLarge: String { text("studio-folder-too-large", "A theme file is at most 256 KiB.") }
    static func folderLink(_ name: String) -> String { text("studio-folder-link", ["name": .text(name)], "\(name) is a link, which is never followed.") }
}
