// A terminal's status as one card over its pane: the status glyph, who it is and what it says. The words of the
// second line ("Needs input · Approve cargo test?", "Working · 3m") are the sidebar's (core's `agent_detail`):
// they arrive made, so the two never disagree. The host decides when it shows (an unseen result, a question).

import DesignSystem
import Foundation

public struct AgentStatusSummary: Equatable, Sendable {
    public var status: StatusKind
    /// "Claude Code", or the program that reports its own status.
    public var name: String
    /// What the second line of the terminal's row says.
    public var detail: String?

    public init(status: StatusKind, name: String, detail: String? = nil) {
        (self.status, self.name, self.detail) = (status, name, detail)
    }

    /// The state in words, the card's heading and what VoiceOver says first.
    public var title: String {
        switch status {
        case .idle: L10n.statusIdle
        case .working: L10n.statusWorking
        case .attention: L10n.statusAttention
        case .done: L10n.statusDone
        case .failed: L10n.statusFailed
        }
    }
}
