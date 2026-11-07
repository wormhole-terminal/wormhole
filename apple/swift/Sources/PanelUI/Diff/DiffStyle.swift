// How a diff's rows are laid out for one file: the code font, the row height, the gutter that fits the largest line
// number (`features/git/diff/render.rs` `RowStyle`). Whole points, so the gutter's edge and every tint after it are
// crisp.

import CoreGraphics

struct DiffStyle {
    var code: CodeFont
    var digits = 3
    var wrap = true
    /// One glyph's advance in the code font, measured once by the view that makes the style.
    var advance: CGFloat

    var fontSize: CGFloat { code.size }
    /// Roomy like an editor with generous spacing, so tinted rows read as bands rather than stripes.
    var rowHeight: CGFloat { max(22, (fontSize * 1.75).rounded()) }
    /// Space around a line number in the gutter: 12 before, 8 after.
    var gutter: CGFloat { (CGFloat(max(digits, 3)) * advance + 20).rounded(.up) }
    /// The +/- column between the numbers and the code.
    static let marker: CGFloat = 16
    /// The bar along a commented line's left edge (`COMMENT_BAR_W`).
    static let commentBar: CGFloat = 2
}
