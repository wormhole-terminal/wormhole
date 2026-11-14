// The sidebar's note after a first run without coding agents (`onboarding/mod.rs` `render_no_agents_note`): what the rows show
// instead of an agent's words and the way to the agents' settings. Its × turns it into one line that offers "Don't show again"
// (kept for good until Settings → Advanced → Reset tips) or just closes it for now. The window decides when it is wanted: not
// once a terminal of the window runs an agent.

import DesignSystem
import SwiftUI
import WordsModel

/// Where the note stands: shown, the line after its × (offering to keep it closed for good), or gone.
public enum AgentsTip: Sendable, Equatable {
    case off, note, offer
}

public struct NoAgentsTip: View {
    let stage: AgentsTip
    let openSettings: () -> Void
    let close: () -> Void
    let never: () -> Void
    let keep: () -> Void
    @Environment(\.tokens) private var tokens

    public init(stage: AgentsTip, openSettings: @escaping () -> Void, close: @escaping () -> Void, never: @escaping () -> Void, keep: @escaping () -> Void) {
        (self.stage, self.openSettings, self.close, self.never, self.keep) = (stage, openSettings, close, never, keep)
    }

    public var body: some View {
        switch stage {
        case .off: EmptyView()
        case .note: note
        case .offer: offer
        }
    }

    private var note: some View {
        HStack(alignment: .top, spacing: Space._2) {
            sparkle(tokens.textMuted)
            VStack(alignment: .leading, spacing: Space._1) {
                Text(Words.text("tip-no-agents-title", fallback: "No coding agents yet")).font(tokens.font(.label)).foregroundStyle(tokens.textPrimary)
                Text(Words.text("tip-no-agents-body", fallback: "Rows show the running command instead. Status glyphs still mark a busy terminal."))
                    .font(tokens.font(.caption)).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
                link(Words.text("tip-no-agents-settings", fallback: "Coding agents settings"), openSettings)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            IconButton(lucide: .x, label: Words.text("tip-close", fallback: "Close tip"), action: close)
        }
        .padding(.leading, Space._4).padding(.trailing, Space._2).padding(.vertical, Space._3)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
    }

    private var offer: some View {
        HStack(spacing: Space._2) {
            sparkle(tokens.textFaint)
            Text(Words.text("tip-closed", fallback: "Tip closed")).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted).lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            link(Words.text("tip-never", fallback: "Don't show again"), never)
            IconButton(lucide: .x, label: Words.text("tip-keep", fallback: "Keep the tip for next time"), action: keep)
        }
        .padding(.leading, Space._4).padding(.trailing, Space._2).padding(.vertical, Space._1)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
    }

    private func sparkle(_ colour: Color) -> some View {
        Image(systemName: "sparkles").font(.system(size: Metric.glyph)).foregroundStyle(colour).accessibilityHidden(true)
    }

    private func link(_ title: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) { Text(title).font(tokens.font(.caption)).foregroundStyle(tokens.primary) }
            .buttonStyle(.plain)
    }
}
