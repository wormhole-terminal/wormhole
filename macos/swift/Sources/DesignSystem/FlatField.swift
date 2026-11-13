// The flat text field (`style::text_field`): a `control.background` well, a little more under the pointer, no frame; focus
// is a soft ring of the primary round it (half strength, the Increase Contrast ring solid with an outline at rest). Parts
// the GPUI field has: a prefix and a suffix, a clear button for a cleanable field (no tab stop, spoken), disabled. The
// editing is the system's `TextField`, so input methods, dictation and the keys stay the system's.

import SwiftUI

public struct FlatField<Prefix: View, Suffix: View>: View {
    let placeholder: String
    @Binding var text: String
    let cleanable: Bool
    let clearLabel: String
    /// Draw the focus ring whatever the focus is (the Studio's gallery cannot focus a specimen); `nil`: the real focus.
    let showFocus: Bool?
    /// Return, or the focus leaving: what was typed is applied (the settings fields).
    let onCommit: (() -> Void)?
    let prefix: Prefix
    let suffix: Suffix
    @FocusState private var focused: Bool
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled
    @Environment(\.increaseContrast) private var increased

    /// How much of the primary the ring shows (`FLAT_RING_ALPHA`); Increase Contrast shows all of it.
    private static var ringAlpha: Double { 0.5 }

    public init(
        _ placeholder: String, text: Binding<String>, cleanable: Bool = false, clearLabel: String = "",
        showFocus: Bool? = nil, onCommit: (() -> Void)? = nil, @ViewBuilder prefix: () -> Prefix, @ViewBuilder suffix: () -> Suffix
    ) {
        (self.placeholder, _text, self.cleanable, self.clearLabel, self.showFocus) = (placeholder, text, cleanable, clearLabel, showFocus)
        self.onCommit = onCommit
        (self.prefix, self.suffix) = (prefix(), suffix())
    }

    public var body: some View {
        let isFocused = enabled && (showFocus ?? focused)
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius, style: .continuous)
        HStack(spacing: Space._2) {
            prefix.foregroundStyle(tokens.textMuted)
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(tokens.font(.body))
                .foregroundStyle(enabled ? tokens.textPrimary : tokens.textDisabled)
                .focused($focused)
                .onSubmit { onCommit?() }
                .onChange(of: focused) { _, now in if !now { onCommit?() } }
            if cleanable, enabled, !text.isEmpty { clearButton }
            suffix.foregroundStyle(tokens.textMuted)
        }
        .padding(.horizontal, Space._2)
        .frame(minHeight: Metric.control)
        .background(isFocused || !enabled || !hovering ? tokens.controlBackground : tokens.controlHover, in: shape)
        .overlay { if increased && !isFocused { shape.strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth) } }
        .overlay { ring(shown: isFocused) }
        .contentShape(shape)
        .onHover { hovering = $0 }
        .marked("input", style: "height control · padding x _2 · radius control · text body")  // text: inspect mode's note, in the scales' names
    }

    /// Round the well, one point wider than a select's ring (a field has no outline to hide it in).
    private func ring(shown: Bool) -> some View {
        let width = tokens.metrics.focusRingWidth + 1
        return Group {
            if shown {
                RoundedRectangle(cornerRadius: tokens.metrics.radius + width, style: .continuous)
                    .strokeBorder(tokens.inputBorderFocus.opacity(increased ? 1 : Self.ringAlpha), lineWidth: width)
                    .padding(-width)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }
    }

    private var clearButton: some View {
        Button { text = "" } label: {
            Image(systemName: "xmark").font(.system(size: Metric.chevron, weight: .bold))
                .foregroundStyle(tokens.textMuted)
                .frame(width: Metric.lead, height: Metric.lead)
                .background(tokens.controlHover, in: Circle())
        }
        .buttonStyle(.plain)
        .focusable(false)
        .accessibilityLabel(Text(clearLabel))
    }
}

extension FlatField where Prefix == EmptyView, Suffix == EmptyView {
    public init(
        _ placeholder: String, text: Binding<String>, cleanable: Bool = false, clearLabel: String = "", showFocus: Bool? = nil,
        onCommit: (() -> Void)? = nil
    ) {
        self.init(placeholder, text: text, cleanable: cleanable, clearLabel: clearLabel, showFocus: showFocus, onCommit: onCommit, prefix: { EmptyView() }, suffix: { EmptyView() })
    }
}
