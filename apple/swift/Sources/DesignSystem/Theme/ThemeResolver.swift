// The seam to Rust for what the bundled dump can't hold: a user's theme file, the primary and branch picks,
// any window opacity. `wormhole-core` does the work (apple/ffi `resolve_theme`); this file only says what is
// asked and reads the answer, which has the dump's shape. The FFI-backed resolver lives in `UniFFISource`,
// so this target builds without Rust, and then those few things fall back (see `ThemeStore`).

import Foundation

public struct ThemeRequest: Sendable, Equatable {
    public var theme: String
    public var themesDirectory: URL?
    public var primary: String
    public var branch: String
    public var increaseContrast: Bool
    public var look: WindowLook
    /// A project's `theme.json` text: the theme in effect in place of `theme`.
    public var projectJSON: String?

    public init(
        theme: String, themesDirectory: URL? = nil, primary: String = "", branch: String = "", increaseContrast: Bool = false,
        look: WindowLook, projectJSON: String? = nil
    ) {
        (self.theme, self.themesDirectory, self.primary, self.branch) = (theme, themesDirectory, primary, branch)
        (self.increaseContrast, self.look, self.projectJSON) = (increaseContrast, look, projectJSON)
    }
}

public protocol ThemeResolving: Sendable {
    /// JSON for one resolved theme (as `resolve_theme` writes it); throws with core's reason.
    func resolve(_ request: ThemeRequest) throws -> Data
    /// JSON array of the user's theme files in a folder: `[{id, name, appearance, error}]`.
    func userThemes(in directory: URL) -> Data
    /// A string that changes when a theme file a pick may come from changes (the folder, a pick that is a path).
    func filesStamp(in directory: URL, picks: [String]) -> String
}

extension ThemeResolving {
    public func filesStamp(in directory: URL, picks: [String]) -> String { "" }
}

/// A user theme file listed in the themes folder. `appearance` is `nil` when the file doesn't resolve.
public struct ThemeSummary: Sendable, Equatable, Identifiable, Decodable {
    public let id: String
    public let name: String
    /// The theme's file, which `theme` holds when it was imported or saved from the Studio.
    public let path: String?
    public let appearance: ThemeAppearance?
    public let error: String?
}

/// The resolver's answer: one theme, already in the contrast and look asked for.
struct ResolvedTheme: Decodable {
    let id: String
    let name: String
    let appearance: ThemeAppearance
    let terminal: TerminalPalette
    let normal: DumpVariant

    init(json: Data) throws { self = try JSONDecoder().decode(ResolvedTheme.self, from: json) }

    var theme: Theme {
        let tokens = normal.tokens(terminal)
        return Theme(id: id, name: name, family: nil, appearance: appearance, normal: tokens, highContrast: tokens)
    }
}
