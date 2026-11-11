// Where a family in a picker comes from (`theme::fonts::FontSource`): the order is the order they are listed in.

public enum FontSource: Int, Sendable, Comparable, CaseIterable {
    /// Ships with the app (JetBrains Mono).
    case bundled
    /// From the user's fonts folder.
    case custom
    case installed

    public static func < (a: Self, b: Self) -> Bool { a.rawValue < b.rawValue }

    public var label: String {
        switch self {
        case .bundled: FontWords.bundled
        case .custom: FontWords.custom
        case .installed: FontWords.installed
        }
    }
}
