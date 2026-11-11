// The search box, focused as the picker opens so typing searches. The keys a list has are caught here, where focus
// is: arrows walk the families, Return picks, Esc clears a search before it lets the popover close.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontSearchField: View {
    @Bindable var model: FontPickerModel
    /// A specimen of the picker must not take the keyboard from whatever is being typed in.
    var autofocus = true
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textFaint)
            TextField(FontWords.searchFonts, text: $model.query)
                .textFieldStyle(.plain).textRole(.body).focused($focused)
                .autocorrectionDisabled()
                .onKeyPress(keys: [.upArrow, .downArrow, .pageUp, .pageDown, .return, .escape], phases: [.down, .repeat], action: press)
            if !model.query.isEmpty {
                Button { model.query = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(tokens.textFaint) }
                    .buttonStyle(.plain).accessibilityLabel(FontWords.searchFonts)
            }
        }
        .padding(.horizontal, Space._3)
        .frame(height: Metric.paletteSearchHeight - Space._4)
        .overlay(alignment: .bottom) { Rectangle().fill(tokens.divider).frame(height: Rule.divider) }
        .onAppear { focused = autofocus }
    }

    private func press(_ key: KeyPress) -> KeyPress.Result {
        switch key.key {
        case .upArrow: move(-1)
        case .downArrow: move(1)
        case .pageUp: move(-Self.page)
        case .pageDown: move(Self.page)
        case .return: model.commitHighlighted()
        case .escape: return model.escape() ? .ignored : .handled
        default: return .ignored
        }
        return .handled
    }

    /// The highlight moves and VoiceOver says the family it landed on (GPUI's `font_picker.rs` announces the label).
    private func move(_ steps: Int) {
        model.move(steps)
        if let family = model.entries.first(where: { $0.id == model.highlighted })?.family { Announcements.post(family) }
    }

    /// A page is the rows the list shows at once.
    private static let page = Int(FontPickerMetric.listHeight / FontPickerMetric.rowHeight)
}
