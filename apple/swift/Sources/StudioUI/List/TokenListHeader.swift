// The head of the token list: the filter field and All / Yours / Low contrast (`studio/token_list.rs`). The
// segments are the system's segmented control, which carries the keyboard and VoiceOver behaviour for free.

import DesignSystem
import StudioModel
import SwiftUI

struct TokenListHeader: View {
    @Binding var filter: TokenFilter
    let yours: Int
    let low: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: Space._2) {
            HStack(spacing: Space._2) {
                Image(systemName: "magnifyingglass").foregroundStyle(tokens.textMuted)
                TextField(StudioWords.filterPlaceholder, text: $filter.query).textFieldStyle(.plain)
                if !filter.query.isEmpty {
                    IconButton(symbol: "xmark.circle.fill", label: "Clear the filter") { filter.query = "" }
                        .frame(width: Metric.controlSmall, height: Metric.controlSmall)
                }
            }
            .padding(.horizontal, Space._2)
            .frame(height: Metric.control)
            .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
            Picker("", selection: $filter.show) {
                Text(StudioWords.showAll).tag(TokenFilter.Show.all)
                Text(StudioWords.showYours(yours)).tag(TokenFilter.Show.yours)
                Text(StudioWords.showLow(low)).tag(TokenFilter.Show.lowContrast)
            }
            .pickerStyle(.segmented).labelsHidden().controlSize(.small)
        }
        .padding(.horizontal, Space._4).padding(.vertical, Space._3)
        .accessibilityElement(children: .contain)
    }
}
