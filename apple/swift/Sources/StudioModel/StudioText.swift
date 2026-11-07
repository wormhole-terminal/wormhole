// The words the model speaks (notices, undo steps, theme names), one function per Fluent message of
// `crates/wormhole-text/locales/en/studio.ftl`, named like its key. English until the FFI hands over the
// catalogue's table; `String(localized:)` keeps each one translatable meanwhile.

import Foundation
import WordsModel

enum StudioText {
    private static func text(_ key: StaticString, _ english: String.LocalizationValue) -> String {
        Words.text(key, fallback: String(localized: key, defaultValue: english))
    }

    static func untitledFrom(_ from: String) -> String { text("studio-untitled-from", "Untitled (from \(from))") }
    static func customName(_ base: String) -> String { text("studio-custom-name", "\(base) Custom") }
    static func copyName(_ name: String, n: Int? = nil) -> String {
        n.map { text("studio-copy-name-n", "\(name) copy \($0)") } ?? text("studio-copy-name", "\(name) copy")
    }
    static let nameFirst = text("studio-name-first", "Give the theme a name first.")
    static func saved(_ path: String) -> String { text("studio-saved", "Saved to \(path).") }
    static func savedAndSet(_ path: String) -> String { text("studio-saved-set", "Saved to \(path) and set as your theme.") }
    static func copySaved(_ name: String, _ path: String) -> String { text("studio-copy-saved", "Saved a copy as “\(name)” to \(path).") }
    static func saveFailed(_ error: String) -> String { text("studio-save-failed", "Could not save: \(error)") }
    static func exportFailed(_ error: String) -> String { text("studio-export-failed", "Could not export: \(error)") }
    static func exported(_ path: String) -> String { text("studio-exported", "Exported to \(path).") }
    static let copiedJSON = text("studio-copied-json", "Copied the theme's JSON.")
    static func nameTaken(_ name: String) -> String {
        text("studio-name-taken", "Another theme is already called “\(name)”. Choose another name.")
    }
    static func nameIsVersion(_ name: String) -> String {
        text("studio-name-is-a-version", "“\(name)” is the name of one version of a theme that comes in several. Choose another name.")
    }
    static func trashFailed(_ name: String) -> String { text("settings-appearance-trash-failed", "Could not move “\(name)” to the Trash.") }
    static func undoChange(_ token: String) -> String { text("studio-undo-change", "Change “\(token)”") }
    static func undoReset(_ token: String) -> String { text("studio-undo-reset", "Reset “\(token)”") }
    static let undoRename = text("studio-undo-rename", "Rename")
    static let undoRevert = text("studio-undo-revert", "Revert to Saved")
    static let undoRestore = text("studio-undo-restore", "Restore Version")
    static func undoMove(_ token: String) -> String { text("studio-undo-move", "Move “\(token)”") }
    static let versionsFirst = text("studio-versions-first", "first save")
    static let versionsCurrent = text("studio-versions-current", "current")
    static let versionsJustNow = text("studio-versions-just-now", "just now")
    static func notPixels(_ name: String, _ value: String) -> String {
        text("studio-not-pixels", "\(name): “\(value)” is not a number of pixels")
    }
}
