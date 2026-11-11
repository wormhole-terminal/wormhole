// The folder picker's words (`folder-picker.ftl`), asked for through the Words seam with the English as the fallback.

import Foundation
import WordsModel

public enum PickerWording {
    public static var name: String { Words.text("folder-picker-name", fallback: "Insert folder path") }
    public static var search: String { Words.text("folder-picker-search", fallback: "Search folders") }
    public static var recent: String { Words.text("folder-picker-recent", fallback: "Recent") }
    public static var hint: String { Words.text("folder-picker-hint", fallback: "↩ insert   ⇥ open   ← up   ⌘⌫ forget recent") }

    public static func note(_ note: PickerNote) -> String {
        switch note {
        case .empty: Words.text("folder-picker-empty", fallback: "No folders")
        case .failed: Words.text("folder-picker-unreadable", fallback: "Can't read this folder")
        case .reading: Words.text("folder-picker-reading", fallback: "Still reading…")
        case .limited(let count):
            Words.text("folder-picker-cut-folders", ["count": .whole(count)], fallback: "Only the first \(count) folders are listed")
        case .searched(let count):
            Words.text(
                "folder-picker-cut-scanned", ["count": .whole(count)],
                fallback: "Only the first \(count) entries were searched, so some folders may be missing")
        case .custom(let text): text
        }
    }
}
