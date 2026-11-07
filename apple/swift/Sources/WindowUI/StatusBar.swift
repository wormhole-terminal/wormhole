// The optional bar along the bottom (hidden by default, `View > Status Bar`): 28 pt on the sidebar's surface
// with a hairline on top, its items 14 apart and starting 12 in. It shows the terminal that needs you (one
// click goes there), the other terminals' states as counts, and the terminal's size at the end. In a narrow
// window the words go and the counts keep a glyph, in a narrower one the size goes too (`StatusSummary.Detail`).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct StatusBar: View {
    let terminals: [TerminalSummary]
    /// The shown terminal's grid, "100×30"; nil when no terminal is drawn.
    let size: String?
    let windowWidth: CGFloat
    let select: @MainActor (TerminalID) -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    /// The room round an item's text, and so the gap between items less twice that.
    private static let itemPad: CGFloat = 4
    private static let gap: CGFloat = 14 - 2 * itemPad
    private static let separatorHeight: CGFloat = 12

    var body: some View {
        let summary = StatusSummary(terminals)
        let detail = StatusSummary.Detail(windowWidth: windowWidth)
        HStack(spacing: Self.gap) {
            if let waiting = summary.attention {
                Button { select(waiting.id) } label: {
                    HStack(spacing: Space._1) {
                        StatusDot(.attention)
                        Text(waiting.title).lineLimit(1)
                        if detail.words { Text("needs input").foregroundStyle(colours.attentionText) }
                    }
                }
                .buttonStyle(StatusItemStyle())
            }
            HStack(spacing: 0) {
                ForEach(Array(summary.rest.enumerated()), id: \.element.id) { index, count in
                    if index > 0 && detail.words { Text("·").foregroundStyle(colours.faint) }
                    Button { select(count.first) } label: { countLabel(count, words: detail.words) }
                        .buttonStyle(StatusItemStyle())
                }
            }
            Spacer(minLength: 0)
            if let size, detail.size {
                HStack(spacing: Space._1) {
                    colours.rule.frame(width: 1, height: Self.separatorHeight)
                    Text(size).padding(.horizontal, Self.itemPad)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Terminal size \(size)")
            }
        }
        .font(TextRole.caption.font.weight(.medium))
        .monospacedDigit()
        .foregroundStyle(colours.muted)
        .padding(.horizontal, Chrome.edgeInset - Self.itemPad)
        .frame(height: Chrome.statusHeight)
        .background { if !look.glassOn { colours.sidebar.opacity(look.effectiveOpacity) } }
        .overlay(alignment: .top) { if !look.glassOn { colours.rule.frame(height: colours.dividerWidth) } }
        .animation(Motion.fade, value: summary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Status bar")
    }

    @ViewBuilder private func countLabel(_ count: StatusSummary.Count, words: Bool) -> some View {
        if words {
            Text(count.words)
        } else {
            HStack(spacing: Space._1) {
                StatusDot(count.activity.status)
                Text("\(count.number)")
            }
        }
    }
}

/// An item of the bar: its metrics (22 pt tall, 4 round the text), the hover wash and the pressed one.
private struct StatusItemStyle: ButtonStyle {
    @State private var hovering = false
    @Environment(\.chrome) private var colours

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .padding(.horizontal, 4)
            .frame(height: 22)
            .background(
                configuration.isPressed || hovering ? colours.hover : .clear,
                in: RoundedRectangle(cornerRadius: colours.radius)
            )
            .onHover { hovering = $0 }
            .contentShape(Rectangle())
    }
}
