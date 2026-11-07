// What only the system can tell a picker: the installed families, their traits, and what a family's faces can
// draw. CoreText answers in `FontPickerUI` (GPUI asks the same questions of CoreText); a stub answers in tests.
// Every call is thread-safe and may take a while, so the library runs them off the main actor.

import Foundation

public protocol SystemFonts: Sendable {
    /// Every installed family, hidden ones included (the rules leave those out).
    func families() -> [String]
    /// Traits for each family the system knows; a family it cannot look up is left out.
    func traits(of families: [String]) -> [String: FontTraits]
    /// Makes font files available to this process, as installed fonts are, without touching the system.
    func register(_ files: [URL])
    /// The faces of a family by style name ("Regular", "Bold Italic"), in the system's order.
    func styles(of family: String) -> [String]
    /// Whether the family itself has a glyph for every scalar (the system's fallback does not count).
    func covers(_ family: String, _ scalars: [Unicode.Scalar]) -> Bool
    /// Whether the family joins sequences like `=>` into one glyph.
    func hasLigatures(_ family: String) -> Bool
}

/// A fixed answer, for tests and previews of a window without a system to ask.
public struct StubSystemFonts: SystemFonts {
    public var installed: [String: FontTraits]
    public var covered: Set<String>
    public var joins: Set<String>

    public init(installed: [String: FontTraits] = [:], covered: Set<String> = [], joins: Set<String> = []) {
        (self.installed, self.covered, self.joins) = (installed, covered, joins)
    }

    public func families() -> [String] { installed.keys.sorted() }
    public func traits(of families: [String]) -> [String: FontTraits] { installed.filter { families.contains($0.key) } }
    public func register(_ files: [URL]) {}
    public func styles(of family: String) -> [String] { ["Regular", "Bold", "Italic", "Bold Italic"] }
    public func covers(_ family: String, _ scalars: [Unicode.Scalar]) -> Bool { covered.contains(family) }
    public func hasLigatures(_ family: String) -> Bool { joins.contains(family) }
}
