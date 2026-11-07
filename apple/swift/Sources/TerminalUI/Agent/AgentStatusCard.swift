// The summary card: a status dot in the lead column, the agent's name with its state, and the second line.
// Needs input and Finished take their text colours (second lines and the status bar use the same); the card is
// the floating surface, so it reads over any terminal. A change of state announces itself to VoiceOver only when it
// needs the person (needs input, failed), as the row does.

import DesignSystem
import SwiftUI

public struct AgentStatusCard: View {
    let summary: AgentStatusSummary
    @Environment(\.tokens) private var tokens

    public init(summary: AgentStatusSummary) { self.summary = summary }

    /// A card is no wider than a notification (the sidebar's default is 350).
    private static let maxWidth: CGFloat = 360

    public var body: some View {
        HStack(alignment: .top, spacing: Space._2) {
            StatusDot(summary.status).frame(height: TextRole.label.lineHeight)
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space._1) {
                    Text(summary.name).font(.system(size: TextRole.label.size, weight: .semibold))
                        .foregroundStyle(tokens.textPrimary)
                    Text(summary.title).font(.system(size: TextRole.label.size, weight: .medium))
                        .foregroundStyle(titleColor)
                }
                .lineLimit(1)
                if let detail = summary.detail {
                    Text(detail).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted).lineLimit(2)
                }
            }
        }
        .padding(.horizontal, Space._3).padding(.vertical, Space._2)
        .frame(maxWidth: Self.maxWidth, alignment: .leading)
        .floatingSurface()
        .fixedSize(horizontal: false, vertical: true)
        .motion(Motion.selection, value: summary)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel([summary.name, summary.title, summary.detail].compactMap { $0 }.joined(separator: ", "))
        .onChange(of: summary.status) { _, status in
            guard status == .attention || status == .failed else { return }
            AccessibilityNotification.Announcement("\(summary.name), \(summary.title)").post()
        }
    }

    private var titleColor: Color {
        switch summary.status {
        case .attention: tokens.agentAttentionText
        case .done: tokens.agentDoneText
        case .failed: tokens.statusDangerText
        case .idle, .working: tokens.textMuted
        }
    }
}
