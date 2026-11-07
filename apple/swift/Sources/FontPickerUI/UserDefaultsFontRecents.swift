// The recents in the app's user defaults (the Swift app's own domain, never the GPUI app's), one key per slot.

import FontPickerModel
import Foundation

public final class UserDefaultsFontRecents: FontRecents, @unchecked Sendable {
    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) { self.defaults = defaults }

    public func families(for slot: String) -> [String] { defaults.stringArray(forKey: Self.key(slot)) ?? [] }

    public func store(_ families: [String], for slot: String) { defaults.set(families, forKey: Self.key(slot)) }

    private static func key(_ slot: String) -> String { "fontPicker.recent.\(slot)" }
}
