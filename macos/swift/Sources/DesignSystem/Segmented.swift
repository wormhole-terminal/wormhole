// A row of choices in one rounded track so they read as one control (`style::segmented`): the chosen one filled like a
// selected chip (`chip.selected`, its text `text.on_selected`), sliding to the next. Split | Unified in a diff, the
// Theme Studio's part-of-the-app filter. The segments are buttons: focus, space and VoiceOver come with them.

import SwiftUI

/// Choices in one rounded track; the chosen one is filled.
public struct Segmented<Value: Hashable>: View {
    let options: [(title: String, value: Value)]
    @Binding var selection: Value
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled
    @Namespace private var space
    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(_ options: [(title: String, value: Value)], selection: Binding<Value>, file: StaticString = #fileID, line: UInt = #line) {
        (self.options, _selection) = (options, selection)
        (self.file, self.line) = (file, line)
    }

    public var body: some View {
        HStack(spacing: 0) {
            ForEach(options.indices, id: \.self) { index in
                segment(options[index].title, on: options[index].value == selection) { selection = options[index].value }
            }
        }
        .padding(2).frame(height: Metric.titleControl)
        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip + 2))
        .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusChip + 2), \.borderStrong)
        .marked("segmented", style: "height titleControl · padding x _2 · radius chip · text caption", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .motion(Motion.selection, value: selection)
        // Held back while disabled (`slider`'s disabled opacity); the track is one control to VoiceOver, each segment a button in it.
        .opacity(enabled ? 1 : 0.5)
        .accessibilityElement(children: .contain)
    }

    private func segment(_ title: String, on: Bool, action: @escaping () -> Void) -> some View {
        Segment(title: title, on: on, space: space, action: action)
    }
}

/// One segment: the chosen one filled like a selected chip; another's text turns primary under the pointer (`segment`).
private struct Segment: View {
    let title: String
    let on: Bool
    let space: Namespace.ID
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled

    var body: some View {
        Button(action: action) {
            Text(title).font(tokens.font(.caption).weight(.medium)).lineLimit(1)
                .foregroundStyle(on ? tokens.textOnSelected : hovering && enabled ? tokens.textPrimary : tokens.textMuted)
                .padding(.horizontal, Space._2).frame(maxHeight: .infinity)
                .background {
                    if on {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).fill(tokens.chipSelected)
                            .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusChip), \.borderStrong)
                            .slidingSelection("segment", in: space)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityAddTraits(on ? .isSelected : [])
    }
}
