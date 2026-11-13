// A row of choices in one rounded track so they read as one control (`style::segmented`): the chosen one filled like a
// selected chip (`chip.selected`, its text `text.on_selected`), sliding to the next. Split | Unified in a diff, the
// Theme Studio's part-of-the-app filter. The segments are buttons: focus, space and VoiceOver come with them.

import SwiftUI

/// Choices in one rounded track; the chosen one is filled.
public struct Segmented<Value: Hashable>: View {
    let options: [(title: String, value: Value)]
    @Binding var selection: Value
    @Environment(\.tokens) private var tokens
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
    }

    private func segment(_ title: String, on: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(tokens.font(.caption).weight(.medium)).lineLimit(1)
                .foregroundStyle(on ? tokens.textOnSelected : tokens.textMuted)
                .padding(.horizontal, Space._2).frame(maxHeight: .infinity)
                .background {
                    if on {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).fill(tokens.chipSelected)
                            .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusChip), \.borderStrong)
                            .matchedGeometryEffect(id: "segment", in: space)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(on ? .isSelected : [])
    }
}
