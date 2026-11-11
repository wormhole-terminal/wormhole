// The families a Mac has for a picker, as plain values: the installed ones (listed first, classified after), the
// bundled one and the user's font files. GPUI loads them the same way, in two steps, so a picker shows what is
// there so far (`fonts.rs` `FontCatalog`).

import Foundation

/// What CoreText says about a family's default face.
public struct FontTraits: Sendable, Equatable {
    public var monospace: Bool
    /// Symbol and ornament fonts: their own face cannot spell their name.
    public var symbolic: Bool
    public init(monospace: Bool, symbolic: Bool) { (self.monospace, self.symbolic) = (monospace, symbolic) }
}

/// A family from the user's font files.
public struct CustomFamily: Sendable, Equatable {
    public let name: String
    public let monospace: Bool
    public init(name: String, monospace: Bool) { (self.name, self.monospace) = (name, monospace) }
}

/// The fonts folder's files and the families they hold (core's scan).
public struct CustomFonts: Sendable, Equatable {
    public var files: [URL]
    public var families: [CustomFamily]
    public var skipped: Int
    public init(files: [URL] = [], families: [CustomFamily] = [], skipped: Int = 0) {
        (self.files, self.families, self.skipped) = (files, families, skipped)
    }
}

public struct FontCatalog: Sendable, Equatable {
    public var installed: [String] = []
    public var bundled: [String] = []
    public var custom: [CustomFamily] = []
    /// Fixed-pitch families, installed and the ones Wormhole loaded; nil until the installed ones are classified.
    public var monospace: Set<String>?
    public var symbolic: Set<String> = []

    public init() {}

    public var customNames: [String] { custom.map(\.name) }
    /// Nothing listed yet: the installed families are still being asked for.
    public var isLoading: Bool { installed.isEmpty && bundled.isEmpty && custom.isEmpty }
    public var isClassifying: Bool { monospace == nil }
    /// A family's own face can spell its name unless it is a symbol font.
    public func drawsOwnName(_ family: String) -> Bool { !symbolic.contains(family) }
    public func isMonospace(_ family: String) -> Bool { monospace?.contains(family) ?? false }
}
