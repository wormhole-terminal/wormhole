// The font code is drawn in (the diff, the viewer, a trust notice's commands): the person's code font from Settings →
// Fonts, or the system's monospace where none is set. The window puts it in the environment; a view asks it for a size.

import AppKit
import SwiftUI

public struct CodeFont: Equatable, Sendable {
    /// The family's name; nil is the system's monospaced font.
    public var family: String?
    public var size: CGFloat
    /// Settings → Fonts' Ligatures (`font_ligatures`), for the code text GPUI draws with `code_font_features`
    /// (`fonts.rs:219`): a project's commands, the project commands window and a git problem's text (`lineFont`). The diff
    /// and the viewer are shaped with the font's own features in both apps (`font`).
    public var ligatures: Bool
    /// Moves when font files were registered or dropped, so what draws with the family looks it up again.
    public var folderRevision: Int

    public init(family: String? = nil, size: CGFloat = 13, ligatures: Bool = true, folderRevision: Int = 0) {
        self.folderRevision = folderRevision
        self.family = family
        self.size = size
        self.ligatures = ligatures
    }

    public func font(_ size: CGFloat? = nil) -> Font {
        let size = size ?? self.size
        guard let family, let face = CTFont.face(of: family, size: size) else { return .system(size: size, design: .monospaced) }
        return Font(face)
    }

    /// The font of a line of code set among the interface's words (a command a project would run, git's own message):
    /// `font`, with `=>` and the like left apart while Ligatures is off.
    public func lineFont(_ size: CGFloat? = nil) -> Font {
        guard !ligatures else { return font(size) }
        return Font(lineFace(size ?? self.size))
    }

    /// The face `lineFont` draws with while Ligatures is off: the family's (the system's monospaced one where there is none)
    /// without its joining features.
    func lineFace(_ size: CGFloat) -> CTFont {
        let face = family.flatMap { CTFont.face(of: $0, size: size) } ?? (NSFont.monospacedSystemFont(ofSize: size, weight: .regular) as CTFont)
        return face.withoutLigatures()
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
