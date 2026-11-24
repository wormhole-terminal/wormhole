// One icon of the app that is not an SF Symbol today, the symbol proposed for it, and the motions offered for it: a row of
// the "SF Symbols" specimen. The id is short and stable so a pick can be named back ("toggle-sidebar: B").
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SidebarUI
import SwiftUI

struct SymbolProposal: Identifiable {
    /// Where today's drawing comes from.
    enum Source {
        /// A Lucide drawing (`LucideView`).
        case lucide(Lucide)
        /// A drawing of the sidebar (`GlyphView`).
        case glyph(Glyph)
        /// The needs-you badge's bar and dot.
        case needsYou
        /// A drawing of the window's own module, which the Studio cannot name: the app hands it over.
        case window(Side)

        enum Side { case sidebar, panel }
    }

    /// How large the icon is drawn where the app draws it.
    enum Size {
        case glyph, title

        var points: CGFloat { self == .glyph ? Metric.glyph : Metric.titleGlyph }
    }

    let id: String
    /// What it says, and where the app draws it, in a few words.
    let meaning: String
    let source: Source
    /// The proposal, or `nil` where no symbol says the same and the drawing stays.
    let symbol: String?
    /// The symbol's filled or other state, where the icon has one.
    var alternate: String?
    /// Why the drawing stays, for a row with no symbol.
    var reason: String?
    var size = Size.glyph
    /// The symbol weight that matches the drawing's stroke: the drawings' lines are 2 of 24 units, which `.regular` is
    /// within a hundredth of an em of; a thicker drawing takes a heavier weight.
    var weight = Font.Weight.regular
    var motions: [SymbolMotion] = []

    var weightName: String { weight == .semibold ? "semibold" : weight == .black ? "black" : "regular" }
}

/// A heading and the rows under it, in the order of the window.
struct SymbolGroup: Identifiable {
    let title: String
    let rows: [SymbolProposal]

    var id: String { title }
}
