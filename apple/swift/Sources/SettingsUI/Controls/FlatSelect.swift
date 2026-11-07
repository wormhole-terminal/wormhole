// A select: the value in a control-fill face with up/down chevrons, a native menu under it (`style` select on
// a card). The menu is `Menu` so keyboard, type-ahead and VoiceOver are the system's. `onOpen` tells the page the
// row got attention.

import DesignSystem
import SettingsModel
import SwiftUI

struct FlatSelect: View {
    let options: [SettingOption]
    let selected: JSONValue
    let label: String
    var width: CGFloat = SettingsMetric.controlWidth
    let onSelect: (JSONValue) -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Menu {
            ForEach(options) { option in
                Button {
                    onSelect(option.value)
                } label: {
                    if option.value == selected { Label(option.label, systemImage: "checkmark") } else { Text(option.label) }
                }
            }
        } label: {
            HStack(spacing: Space._2) {
                Text(options.first { $0.value == selected }?.label ?? selected.string ?? "")
                    .font(.system(size: TextRole.label.size)).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                Image(systemName: "chevron.up.chevron.down").font(.system(size: Metric.chevron, weight: .semibold))
                    .foregroundStyle(tokens.textMuted)
            }
            .padding(.horizontal, Space._3)
            .frame(width: width, height: Metric.control)
            .background(hovering ? tokens.controlHover : tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .foregroundStyle(tokens.textPrimary)
            .contentShape(Rectangle())
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        .onHover { hovering = $0 }
        .accessibilityLabel(label)
    }
}
