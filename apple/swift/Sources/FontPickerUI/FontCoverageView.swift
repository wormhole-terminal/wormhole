// What the highlighted font draws itself and what it leaves to the system's fallback, as chips: the glyph groups a
// terminal needs, ligatures, and the bold and italic faces the terminal asks for. A font that lacks a group still
// works (CoreText fills in), but the filled-in glyphs may not line up with the cell grid, which is what this warns of.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontCoverageView: View {
    let coverage: FontCoverage

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Text(FontWords.coverage).font(TextRole.caption.font.weight(.semibold)).foregroundStyle(Palette.textSecondary)
            FlowLayout(spacing: Space._1) {
                ForEach(GlyphGroup.all) { group in
                    CoverageChip(label: group.label, ok: coverage.drawn[group.id] ?? false, bad: FontWords.fallsBack)
                }
                CoverageChip(label: FontWords.ligatures, ok: coverage.ligatures, bad: FontWords.fallsBack, none: true)
                CoverageChip(label: FontWords.bold, ok: coverage.hasBold, bad: FontWords.noBold)
                CoverageChip(label: FontWords.italic, ok: coverage.hasItalic, bad: FontWords.noItalic)
            }
        }
        .motion(Motion.selection, value: coverage)
    }
}

private struct CoverageChip: View {
    let label: String
    let ok: Bool
    let bad: String
    /// A font without ligatures is not at fault: the chip says "none" rather than warning.
    var none = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._1) {
            Image(systemName: ok ? "checkmark.circle.fill" : none ? "minus.circle" : "exclamationmark.triangle")
                .font(.system(size: Metric.chevron + 2))
            Text(label).lineLimit(1)
        }
        .font(TextRole.caption.font)
        .foregroundStyle(ok ? tokens.textPrimary : tokens.textFaint)
        .padding(.horizontal, Space._2).frame(height: Metric.badgeHeight + Space._1)
        .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: Radius.chip))
        .help(ok ? FontWords.drawn : bad)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(label): \(ok ? FontWords.drawn : bad)")
    }
}
