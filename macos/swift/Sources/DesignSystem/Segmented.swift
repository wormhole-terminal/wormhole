// A row of choices in one rounded track so they read as one control (`style::segmented`): the chosen one filled like a
// selected chip (`chip.selected`, its text `text.on_selected`), sliding to the next. Split | Unified in a diff, the
// Theme Studio's part-of-the-app filter. One Tab stop, walked with the arrows (and Home, End), wrapping round, the ring
// round the track while it has the keyboard; VoiceOver hears a radio group of its choices, as AppKit's segmented control
// says it (`form_segments`: `RadioGroup` with a `RadioButton` each).

import SwiftUI

/// Choices in one rounded track; the chosen one is filled.
public struct Segmented<Value: Hashable>: View {
    let options: [(title: String, value: Value)]
    @Binding var selection: Value
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled
    @Namespace private var space
    @FocusState private var focused: Bool
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
        // One stop, taken only by keyboard navigation (a click picks a segment and leaves the keyboard where it was).
        .focusable(enabled, interactions: .activate)
        .focused($focused)
        .focusEffectDisabled()
        .keyboardFocusRing(focused, radius: tokens.metrics.radiusChip + 2, outset: IconButtonMark.ringOffset + tokens.metrics.focusRingWidth)
        .onKeyPress(keys: [.leftArrow, .upArrow, .rightArrow, .downArrow, .home, .end]) { press in step(press.key) }
        // Held back while disabled (`slider`'s disabled opacity).
        .opacity(enabled ? 1 : 0.5)
        .accessibilityRepresentation {
            Picker(selection: $selection) {
                ForEach(options.indices, id: \.self) { index in Text(options[index].title).tag(options[index].value) }
            } label: {
                EmptyView()
            }
            .pickerStyle(.segmented)
        }
    }

    /// An arrow picks the segment before or after the chosen one, wrapping round; Home and End the first and the last.
    private func step(_ key: KeyEquivalent) -> KeyPress.Result {
        guard !options.isEmpty else { return .ignored }
        let count = options.count
        let at = options.firstIndex { $0.value == selection } ?? 0
        let next: Int
        switch key {
        case .leftArrow, .upArrow: next = (at + count - 1) % count
        case .rightArrow, .downArrow: next = (at + 1) % count
        case .home: next = 0
        case .end: next = count - 1
        default: return .ignored
        }
        selection = options[next].value
        return .handled
    }

    private func segment(_ title: String, on: Bool, action: @escaping () -> Void) -> some View {
        Segment(title: title, on: on, space: space, action: action)
    }
}

/// One segment: the chosen one filled like a selected chip; another's text turns primary under the pointer (`segment`). A
/// click picks it; it takes no keyboard stop of its own (the track is the stop) and VoiceOver hears the track's picker.
private struct Segment: View {
    let title: String
    let on: Bool
    let space: Namespace.ID
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled

    var body: some View {
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
            .onTapGesture { if enabled { action() } }
            .onHover { hovering = $0 }
    }
}
