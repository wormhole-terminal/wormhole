// The search field over the list: the flat text field of the GPUI app (`style::text_field`: `control.background`, a
// little more under the pointer, no outline, a soft ring of `input.border.focus` while it has the keyboard; Increase
// Contrast keeps an outline) with a magnifier, a round clear button and the count at its end. A plain `TextField`: the
// system gives the caret, selection, input methods and VoiceOver. Esc, Return and ↓ are the list's to answer.

import DesignSystem
import SwiftUI

struct SidebarSearchField: View {
    @Binding var text: String
    let found: Int
    let total: Int
    var focus: FocusState<Bool>.Binding
    let exit: () -> Void
    let submit: () -> Void
    let toList: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    /// How much of the primary the focus ring shows (`FLAT_RING_ALPHA`), and how much wider than a select's it is
    /// (`RING_EXTRA`): a flat field has no outline to hide it in.
    private static let ringAlpha = 0.5
    private static let ringExtra: CGFloat = 1
    /// The round clear glyph (`CLEAR_SIZE`).
    private static let clearSize: CGFloat = 16

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius) }

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            TextField(SidebarText.searchPlaceholder, text: $text)
                .textFieldStyle(.plain).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textPrimary)
                .focused(focus)
                .onKeyPress(.escape) { exit(); return .handled }
                .onKeyPress(.downArrow) { toList(); return .handled }
                .onSubmit(submit)
                .accessibilityLabel(SidebarText.searchPlaceholder)
                .accessibilityHint(SidebarText.searchHint)
            if !text.isEmpty { trailing }
        }
        .padding(.horizontal, Space._2)
        .frame(height: Metric.control)
        .background(shape.fill(tokens.controlBackground))
        .overlay { if hovering && !focus.wrappedValue { shape.fill(tokens.controlHover) } }
        .overlay { edge }
        .contentShape(shape)
        .onTapGesture { focus.wrappedValue = true }
        .onHover { hovering = $0 }
        .animation(reduceMotion ? Motion.fade : Motion.press, value: focus.wrappedValue)
        .animation(reduceMotion ? Motion.fade : Motion.press, value: hovering)
    }

    /// The count, then the round clear button.
    private var trailing: some View {
        HStack(spacing: Space._1) {
            Text(SidebarText.found(found, of: total)).font(.system(size: TextRole.caption.size)).monospacedDigit()
                .foregroundStyle(tokens.textMuted)
                .accessibilityLabel(SidebarText.foundSpoken(found, of: total))
            Button { text = "" } label: {
                Image(systemName: "xmark.circle.fill").font(.system(size: Self.clearSize)).foregroundStyle(tokens.textMuted)
                    .frame(width: Metric.controlSmall, height: Metric.controlSmall).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .help(SidebarText.searchClear)
            .accessibilityLabel(SidebarText.searchClear)
        }
        .transition(.opacity)
    }

    /// Nothing at rest; the ring while focused; an outline at rest under Increase Contrast, the ring solid then.
    @ViewBuilder private var edge: some View {
        let increased = contrast == .increased
        let width = tokens.metrics.focusRingWidth + Self.ringExtra
        if focus.wrappedValue {
            shape.strokeBorder(tokens.inputBorderFocus.opacity(increased ? 1 : Self.ringAlpha), lineWidth: increased ? width + 1 : width)
        } else if increased {
            shape.strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth)
        }
    }
}
