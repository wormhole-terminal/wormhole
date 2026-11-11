// The optional bar along the bottom (hidden by default, `View > Status Bar`): 28 pt on the sidebar's surface
// with a hairline on top, its items 14 apart and starting 12 in. It shows the terminal that needs you (one
// click goes there), the other terminals' states as counts, and the terminal's size at the end. In a narrow
// window the words go and the counts keep a glyph, in a narrower one the size goes too (`StatusSummary.Detail`).

import DesignSystem
import PanesUI
import SwiftUI
import WordsModel
import WorkspaceModel

struct StatusBar: View {
    let terminals: [TerminalSummary]
    /// The shown terminal, whose state and size end the bar; the size is its grid, "100×30", nil when none is drawn.
    let selected: TerminalSummary?
    /// What runs in front of the shown terminal (the title row's last chip says it too): the size item's first word.
    var program: String?
    /// The window's profile when it is not the one Wormhole is in.
    var profile: ProfileChoice?
    let size: String?
    /// Where the shown terminal runs; nil while persistent terminals are off (the bar then says nothing of it).
    var hosting: HostingState?
    let windowWidth: CGFloat
    /// How long ago a terminal got into its state, in the core's words (nil in the sample).
    var waited: (UInt64) -> Waited? = { _ in nil }
    let select: @MainActor (TerminalID) -> Void
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look
    @Environment(WindowFocus.self) private var focus

    /// The gap between items: 14 from one's text to the next's, less the room each has round its text.
    private static let gap: CGFloat = 14 - 2 * Chrome.itemPad

    var body: some View {
        let summary = StatusSummary(terminals)
        let detail = StatusSummary.Detail(windowWidth: windowWidth)
        HStack(spacing: Self.gap) {
            if let profile, detail.words {
                HStack(spacing: Space._1) {
                    Circle().fill(profile.colour ?? colours.primary).frame(width: Metric.statusDot, height: Metric.statusDot)
                    Text(profile.name).lineLimit(1)
                }
                .padding(.horizontal, Chrome.itemPad)
                .help(
                    Words.text(
                        "window-status-profile-edge", ["profile": .text(profile.name)],
                        fallback: "This window is in \(profile.name), not the profile Wormhole is in"))
            }
            if let waiting = summary.attention {
                Button { select(waiting.id) } label: {
                    HStack(spacing: Space._1) {
                        StatusDot(.attention)
                        Text(waiting.title).lineLimit(1)
                        if let since = waiting.since, let waited = waited(since) {
                            WaitedWords(prefix: detail.words ? Words.text("window-state-needs-input", fallback: "needs input") + " · " : "", first: waited, waited: { self.waited(since) })
                                .foregroundStyle(colours.attentionText)
                        } else if detail.words {
                            Text(Words.text("window-state-needs-input", fallback: "needs input")).foregroundStyle(colours.attentionText)
                        }
                    }
                }
                .buttonStyle(StatusItemStyle(tint: colours.attentionTint))
                .help(summary.attentionTooltip ?? "")
                .focusRing(focus.ring(.statusBar, 0), radius: colours.radius)
            }
            HStack(spacing: 0) {
                ForEach(Array(summary.rest.enumerated()), id: \.element.id) { index, count in
                    if index > 0 && detail.words { Text("·").foregroundStyle(colours.faint) }
                    Button { select(count.first) } label: { countLabel(count, words: detail.words) }
                        .buttonStyle(StatusItemStyle())
                        .focusRing(focus.ring(.statusBar, index + (summary.attention == nil ? 0 : 1)), radius: colours.radius)
                }
            }
            Spacer(minLength: 0)
            if detail.size, !look.probeItems.isEmpty { StatusProbeItems(items: look.probeItems) }
            if let selected, let words = selected.activity.stateWords {
                separated { stateText(selected, words).foregroundStyle(selected.activity.stateColour(colours)) }
            }
            if let hosting, StatusHosting.words(hosting) != nil {
                separated { StatusHostingItem(state: hosting, word: detail >= .numbers) }
            }
            if let selected, let size, detail.size {
                separated { Text("\(program ?? selected.title) · \(size)") }
                    .accessibilityLabel(
                        Words.text(
                            "window-status-size-spoken", ["title": .text(program ?? selected.title), "size": .text(size)],
                            fallback: "\(program ?? selected.title), terminal size \(size)"))
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
        .accessibilityLabel(Words.text("window-status-bar", fallback: "Status bar"))
    }

    /// "working · 4m", "done · 3m", "failed · 2m", "needs input", "exited": how long in whole minutes, as the title row says it.
    @ViewBuilder private func stateText(_ terminal: TerminalSummary, _ words: String) -> some View {
        if [.working, .finished, .failed].contains(terminal.activity), let since = terminal.since, let first = waited(since) {
            WaitedWords(prefix: words + " · ", first: first, waited: { self.waited(since) })
        } else {
            Text(words)
        }
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

/// "needs input · 2m": the words move with time, so a one-shot timer, armed for the moment they next read differently,
/// reads them again (never a poll).
private struct WaitedWords: View {
    let prefix: String
    let first: Waited
    let waited: () -> Waited?
    @State private var current: Waited?

    var body: some View {
        let shown = current ?? first
        Text(prefix + shown.words)
            .task(id: shown) {
                try? await Task.sleep(for: shown.next)
                if !Task.isCancelled { current = waited() }
            }
    }
}
