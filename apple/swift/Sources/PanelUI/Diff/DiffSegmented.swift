// "Split | Unified" (`style::segmented`): the segments sit in one rounded track so they read as one control, the
// chosen one filled like a selected chip (`chip.selected`, its text `text.on_selected`).

import DesignSystem
import SwiftUI

struct DiffSegmented: View {
    @Binding var split: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.panelMotion) private var motion
    @Namespace private var space

    var body: some View {
        HStack(spacing: 0) {
            segment("Split", on: split) { split = true }
            segment("Unified", on: !split) { split = false }
        }
        .padding(2).frame(height: Metric.titleControl)
        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip + 2))
        .animation(motion.snap, value: split)
    }

    private func segment(_ title: String, on: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(TextRole.caption.font.weight(.medium))
                .foregroundStyle(on ? tokens.textOnSelected : tokens.textMuted)
                .padding(.horizontal, Space._2).frame(maxHeight: .infinity)
                .background {
                    if on {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).fill(tokens.chipSelected)
                            .matchedGeometryEffect(id: "segment", in: space)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(on ? .isSelected : [])
    }
}

/// A shortcut as a key cap (`style::keycap`): a flat `control.background` pill, 24 tall in a title row.
struct KeyCap: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(TextRole.caption.font.weight(.medium).monospacedDigit()).foregroundStyle(tokens.textPrimary)
            .padding(.horizontal, Space._1).frame(minWidth: Metric.keyCapMinWidth, minHeight: Metric.titleControl)
            .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }
}
