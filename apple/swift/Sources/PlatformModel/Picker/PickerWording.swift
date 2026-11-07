// The folder picker's words (`folder-picker.ftl`), in English until the FFI's text table exists: one function per
// module, keys named like the Fluent ones.

import Foundation

public enum PickerWording {
    public static let name = "Insert folder path"
    public static let search = "Search folders"
    public static let recent = "Recent"
    public static let hint = "↩ insert   ⇥ open   ← up   ⌘⌫ forget recent"

    public static func note(_ note: PickerNote) -> String {
        switch note {
        case .empty: "No folders"
        case .failed: "Can't read this folder"
        case .reading: "Still reading…"
        case .limited(let count): "Only the first \(count) folders are listed"
        case .searched(let count): "Only the first \(count) entries were searched, so some folders may be missing"
        case .custom(let text): text
        }
    }
}
