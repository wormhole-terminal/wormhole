// What the Studio is editing and where it lives: the person's own theme file, edited in place, or a copy of
// another theme (a built-in, one in another format, the one in use) that has no file yet and asks for a name on
// its first save. The Swift Studio edits and writes the documented format (themes.md), the one every release reads.

import Foundation

public enum ThemeSubject: Sendable, Equatable {
    case own(URL)
    case untitled(from: String)

    public var url: URL? { if case .own(let url) = self { url } else { nil } }
}
