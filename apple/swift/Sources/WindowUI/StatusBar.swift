// The optional bar along the bottom (hidden by default, `View > Status Bar`): 28 pt on the sidebar's surface
// with a hairline on top, its items 14 apart and starting 12 in. It shows the terminal that needs you (one
// click goes there), the other terminals' states as counts, and the terminal's size at the end. In a narrow
// window the words go and the counts keep a glyph, in a narrower one the size goes too (`StatusSummary.Detail`).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct StatusBar: View {
    let terminals: [TerminalSummary]
    /// The shown terminal, whose state and size end the bar; the size is its grid, "100×30", nil when none is drawn.
    let selected: TerminalSummary?
    let size: String?
    let windowWidth: CGFloat
    let select: @MainActor (TerminalID) -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    /// The gap between items: 14 from one's text to the next's, less the room each has round its text.
    private static let gap: CGFloat = 14 - 2 * Chrome.itemPad

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
                .buttonStyle(StatusItemStyle(tint: colours.attentionTint))
            }
            HStack(spacing: 0) {
                ForEach(Array(summary.rest.enumerated()), id: \.element.id) { index, count in
                    if index > 0 && detail.words { Text("·").foregroundStyle(colours.faint) }
                    Button { select(count.first) } label: { countLabel(count, words: detail.words) }
                        .buttonStyle(StatusItemStyle())
                }
            }
            Spacer(minLength: 0)
            if let words = selected?.activity.stateWords, detail.words {
                separated { Text(words).foregroundStyle(selected?.activity.stateColour(colours) ?? colours.muted) }
            }
            if let selected, let size, detail.size {
                separated { Text("\(selected.title) · \(size)") }
                    .accessibilityLabel("\(selected.title), terminal size \(size)")
            }
        }
        .font(TextRole.caption.font.weight(.medium))
        .monospacedDigit()
        .foregroundStyle(colours.muted)
        .padding(.horizontal, Chrome.edgeInset - Chrome.itemPad)
        .frame(height: Chrome.statusHeight)
        .background { if !look.glassOn { look.surface(colours.sidebar) } }
        .overlay(alignment: .top) { if !look.glassOn { colours.rule.frame(height: colours.borderWidth) } }
        .animation(Motion.fade, value: summary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Status bar")
    }

    /// An item read, not clicked, after a hairline.
    private func separated<Item: View>(@ViewBuilder _ item: () -> Item) -> some View {
        HStack(spacing: Space._1) {
            colours.rule.frame(width: colours.borderWidth, height: Chrome.statusSeparator)
            item().padding(.horizontal, Chrome.itemPad)
        }
        .accessibilityElement(children: .combine)
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
    /// A fill the item wears at rest: the needs-you button's attention tint.
    var tint: Color = .clear
    @State private var hovering = false
    @Environment(\.chrome) private var colours

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .padding(.horizontal, Chrome.itemPad)
            .frame(height: Chrome.statusItem)
            .background(
                configuration.isPressed || hovering ? colours.bandHover : tint,
                in: RoundedRectangle(cornerRadius: colours.radius)
            )
            .onHover { hovering = $0 }
            .contentShape(Rectangle())
    }
}
