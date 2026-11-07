// How a diff's rows are laid out for one file: the code font, the row height, the gutter that fits the largest
// line number (`features/git/diff/render.rs` `RowStyle`). Whole points, so the gutter's edge and every tint after
// it are crisp.

import CoreGraphics

struct DiffStyle {
    var fontSize: CGFloat = 13
    var digits = 3
    var wrap = true

    /// Roomy like an editor with generous spacing, so tinted rows read as bands rather than stripes.
    var rowHeight: CGFloat { max(22, (fontSize * 1.75).rounded()) }
    /// Roughly the advance of a monospace glyph, in ems.
    var charWidth: CGFloat { fontSize * 0.6 }
    /// Space around a line number in the gutter: 12 before, 8 after.
    var gutter: CGFloat { (CGFloat(max(digits, 3)) * charWidth + 20).rounded(.up) }
    /// The +/- column between the numbers and the code.
    static let marker: CGFloat = 16
}
