// The select of a font row: the family in its own face with a "Bundled" tag, or, while the row follows another
// font, what it follows in the interface font with the font it comes to faint at the right.

import DesignSystem
import FontPickerModel
import FontPickerUI
import SwiftUI

struct FontSelectFace: View {
    let picker: FontPickerModel
    let resolved: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        SelectFace(width: FontPickerMetric.selectWidth) {
            if picker.isInherited, let label = picker.slot.inheritLabel {
                HStack(spacing: Space._2) {
                    Text(label).font(.system(size: TextRole.label.size)).lineLimit(1)
                    Text(Self.shown(resolved)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                        .lineLimit(1).frame(maxWidth: .infinity, alignment: .trailing)
                }
            } else {
                HStack(spacing: Space._2) {
                    Text(picker.selected).font(FontFace.font(picker.selected, size: TextRole.label.size)).lineLimit(1)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    if picker.selected == picker.library.rules.bundledFamily { TintChip(FontWords.bundled) }
                }
            }
        }
    }

    /// The system font's own name, as GPUI says it (`SF Pro`).
    private static func shown(_ family: String) -> String { family == FontFace.system ? "SF Pro" : family }
}
