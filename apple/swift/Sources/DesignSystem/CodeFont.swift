// The font code is drawn in (the diff, the viewer, a trust notice's commands): the person's code font from Settings →
// Fonts, or the system's monospace where none is set. The window puts it in the environment; a view asks it for a size.

import AppKit
import SwiftUI

public struct CodeFont: Equatable, Sendable {
    /// The family's name; nil is the system's monospaced font.
    public var family: String?
    public var size: CGFloat
    /// Moves when font files were registered or dropped, so what draws with the family looks it up again.
    public var folderRevision: Int

    public init(family: String? = nil, size: CGFloat = 13, folderRevision: Int = 0) {
        self.folderRevision = folderRevision
        self.family = family
        self.size = size
    }

    public func font(_ size: CGFloat? = nil) -> Font {
        let size = size ?? self.size
        guard let family, let face = CTFont.face(of: family, size: size) else { return .system(size: size, design: .monospaced) }
        return Font(face)
    }

    /// The advance of one glyph at the font's size, which the gutter and the wrap are measured with.
    @MainActor
    public var advance: CGFloat {
        let ns = family.flatMap { NSFont(name: $0, size: size) } ?? .monospacedSystemFont(ofSize: size, weight: .regular)
        return ns.advancement(forGlyph: ns.glyph(withName: "zero")).width
    }
}

extension EnvironmentValues {
    @Entry public var codeFont = CodeFont()
    /// Columns a tab stands for in the diff and the viewer (`tab_width`).
    @Entry public var tabWidth = 4
}
