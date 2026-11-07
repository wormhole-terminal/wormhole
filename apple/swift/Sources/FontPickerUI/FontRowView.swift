// One family: its name in its own face (a symbol font in the interface font, since its glyphs cannot spell the
// name), the tag of a monospace family in a list that is not monospace only, and a check on the one in use.
// A row that follows another font ("Same as terminal") is muted.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontRowView: View {
    let model: FontPickerModel
    let entry: FontEntry
    let highlight: Namespace.ID
    @Environment(\.tokens) private var tokens

    var body: some View {
        Row(isSelected: model.highlighted == entry.id, selection: highlight) {
            model.choose(entry)
        } content: { _ in
            HStack(spacing: Space._2) {
                label
                Spacer(minLength: Space._2)
                if showsMonospaceTag {
                    Text(FontWords.monospaceTag).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                }
                if model.isSelected(entry) {
                    Image(systemName: "checkmark").font(.system(size: Metric.glyph, weight: .semibold))
                        .foregroundStyle(tokens.textMuted)
                }
            }
            .frame(height: FontPickerMetric.rowHeight)
        }
        .onHover { over in if over { model.hover(entry) } }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(spokenName)
        .accessibilityAddTraits(model.isSelected(entry) ? [.isButton, .isSelected] : .isButton)
    }

    @ViewBuilder private var label: some View {
        switch entry {
        case .inherit(let text):
            Text(text).font(.system(size: TextRole.heading.size)).foregroundStyle(tokens.textMuted).lineLimit(1)
        case .family(let choice, _):
            let own = model.catalog.drawsOwnName(choice.name)
            Text(choice.name).font(own ? FontFace.font(choice.name, size: TextRole.heading.size) : .system(size: TextRole.heading.size))
                .foregroundStyle(tokens.textPrimary).lineLimit(1)
        case .caption: EmptyView()
        }
    }

    /// Monospace is a fact worth saying where the list also holds proportional fonts.
    private var showsMonospaceTag: Bool {
        guard case .family(let choice, _) = entry else { return false }
        return !model.monospaceOnly && model.catalog.isMonospace(choice.name)
    }

    private var spokenName: String {
        switch entry {
        case .inherit(let text): text
        case .family(let choice, _): showsMonospaceTag ? "\(choice.name), \(FontWords.monospaceTag)" : choice.name
        case .caption: ""
        }
    }
}
