// The notice chip's capsule (`platform/macos/notice.rs`): Liquid Glass with an SF Symbol and one line, and, on a
// warning that stays, the ⓘ and × at the trailing end. Reduce Transparency gives it a plain fill and an edge.

import DesignSystem
import SwiftUI

struct NoticeChip: View {
    let notice: Notice
    let center: NoticeCenter
    @Environment(\.tokens) private var tokens
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    private typealias M = NoticeChipMetrics

    var body: some View {
        HStack(spacing: M.gap) {
            Image(systemName: notice.symbol)
                .font(.system(size: M.icon * 0.85))
                .frame(width: M.icon, height: M.icon)
                .foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            Text(notice.words)
                .font(.system(size: TextRole.label.size))
                .foregroundStyle(tokens.textPrimary)
                .lineLimit(1)
                .contentTransition(.opacity)
            if notice.hasInfo { button("info.circle", L10n.noticeInfo, center.clickInfo) }
            if notice.isSticky { button("xmark", L10n.noticeClose, center.dismiss) }
        }
        .padding(.leading, M.pad)
        .padding(.trailing, notice.isSticky ? M.endPad : M.pad)
        .frame(height: M.height)
        .background { surface }
        .contentShape(Capsule())
        .onTapGesture { center.clickBody() }
        .animation(.easeInOut(duration: NoticeTiming.swap), value: notice)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(notice.words)
        .accessibilityAddTraits(notice.isSticky ? .isButton : [])
    }

    @ViewBuilder private var surface: some View {
        if reduceTransparency || ThemeStore.shared.reduceTransparency {
            Capsule().fill(tokens.surfaceElevated)
                .overlay { Capsule().strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
        } else {
            Capsule().fill(.clear).glassEffect(.regular, in: .capsule)
        }
    }

    private func button(_ symbol: String, _ label: String, _ action: @escaping () -> Void) -> some View {
        ChipButton(symbol: symbol, label: label, action: action)
    }
}

/// ⓘ and ×: a 20 pt square, a 11 pt picture, a circle that lights under the pointer.
private struct ChipButton: View {
    let symbol: String
    let label: String
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: NoticeChipMetrics.buttonIcon, weight: .semibold))
                .frame(width: NoticeChipMetrics.button, height: NoticeChipMetrics.button)
                .background(hovering ? tokens.stateHover : .clear, in: Circle())
        }
        .buttonStyle(.plain)
        .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
        .onHover { hovering = $0 }
        .motion(Motion.press, value: hovering)
        .help(label)
        .accessibilityLabel(label)
    }
}
