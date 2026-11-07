// The search field under the traffic lights (`style::text_field` with a magnifier, the result count and a clear
// button). Esc clears the search, then (with it empty) lets the window close; Return opens the first result.
// A plain `TextField`: the system gives the caret, selection, IME and VoiceOver.

import DesignSystem
import SettingsModel
import SwiftUI

struct NavSearchField: View {
    let model: SettingsModel
    var focus: FocusState<SettingsFocus?>.Binding
    @Environment(\.tokens) private var tokens

    var body: some View {
        @Bindable var model = model
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
            TextField(SettingsText.searchSettings, text: $model.query)
                .textFieldStyle(.plain).font(.system(size: TextRole.body.size))
                .focused(focus, equals: .search)
                .onSubmit { if let first = model.hits.first { model.jump(to: first) } }
                .accessibilityLabel(SettingsText.searchSettings)
            if model.isSearching {
                Text("\(model.hits.count)").font(.system(size: TextRole.caption.size)).monospacedDigit().foregroundStyle(tokens.textMuted)
                IconButton(symbol: "xmark.circle.fill", label: LocalizedStringKey(SettingsText.clearSearch)) { model.query = "" }
                    .frame(width: Metric.controlSmall, height: Metric.controlSmall)
            }
        }
        .padding(.horizontal, Space._2)
        .frame(height: Metric.control)
        .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay {
            RoundedRectangle(cornerRadius: tokens.metrics.radius)
                .strokeBorder(focus.wrappedValue == .search ? tokens.inputBorderFocus : tokens.inputBorder, lineWidth: tokens.metrics.borderWidth)
        }
    }
}
