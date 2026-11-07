// A few options in one segmented control (`style::segmented_form`): a quiet trough, the current option raised
// on the selected control fill. The selection is one shape that slides between options (and cuts under Reduce
// Motion). `onHover` reports the option under the pointer so a page can preview it.

import DesignSystem
import SettingsModel
import SwiftUI

struct FlatSegmented: View {
    let options: [SettingOption]
    let selected: JSONValue
    let label: String
    let onSelect: (JSONValue) -> Void
    var onHover: (JSONValue?) -> Void = { _ in }
    @Environment(\.tokens) private var tokens
    @Namespace private var slide

    var body: some View {
        HStack(spacing: 0) {
            ForEach(options) { option in
                Segment(option: option, isSelected: option.value == selected, slide: slide) { onSelect(option.value) }
                    .onHover { onHover($0 ? option.value : nil) }
            }
        }
        .padding(2)
        .background(tokens.controlBackground.opacity(0.6), in: RoundedRectangle(cornerRadius: tokens.metrics.radius + 2))
        .motion(Motion.selection, value: selected)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
    }
}

private struct Segment: View {
    let option: SettingOption
    let isSelected: Bool
    let slide: Namespace.ID
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                if let icon = option.icon { Image(systemName: icon).font(.system(size: Metric.glyph)) }
                Text(option.label).font(.system(size: TextRole.label.size, weight: .medium)).lineLimit(1)
            }
            .padding(.horizontal, Space._3)
            .frame(height: Metric.control - 4)
            .foregroundStyle(isSelected ? tokens.textPrimary : tokens.textMuted)
            .background {
                if isSelected {
                    RoundedRectangle(cornerRadius: tokens.metrics.radius)
                        .fill(tokens.controlSelected)
                        .matchedGeometryEffect(id: "selection", in: slide)
                } else if hovering {
                    RoundedRectangle(cornerRadius: tokens.metrics.radius).fill(tokens.controlHover.opacity(0.5))
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(option.label)
        .accessibilityAddTraits(isSelected ? [.isSelected, .isButton] : .isButton)
    }
}
