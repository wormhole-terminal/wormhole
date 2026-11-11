// "Coding agents": the agents on PATH, each with its own Install, then the one that is not found, the notice about
// what Start does not do, the notification switch and the folder's trust (`agents_step.rs`).

import AuxModel
import DesignSystem
import SwiftUI

struct AgentsStep: View {
    let session: OnboardingSession
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let agents = session.agents {
                Rule()
                ForEach(agents) { agent in
                    AgentRow(agent: agent) { session.send(.install(agent.id)) }
                    Rule()
                }
                if session.othersFound > 0 {
                    HStack(spacing: Space._3) {
                        Monogram(name: "?", dimmed: true)
                        Text(AuxText.notFound(more: session.othersFound)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                    }
                    .padding(.horizontal, Space._6).padding(.vertical, Space._2)
                    Rule()
                }
                if let problem = session.problem {
                    Text(problem).font(TextRole.caption.font).foregroundStyle(tokens.statusDangerText)
                        .padding(.horizontal, Space._6).padding(.top, Space._3)
                }
                Text(AuxText.installsNothing).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    .fixedSize(horizontal: false, vertical: true).padding(.horizontal, Space._6).padding(.vertical, Space._3)
                NotifyToggle(label: AuxText.notifyBackground, isOn: Bindable(session.preferences).notifications)
                if let card = session.trustCard {
                    TrustCardView(card: card, trust: { session.send(.trust(session.flow.folder ?? URL(fileURLWithPath: "/"))) }, later: session.laterOnTrust)
                }
            } else {
                Text(AuxText.lookingForAgents).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                    .padding(.horizontal, Space._6).padding(.vertical, Space._3)
            }
        }
    }
}

/// A line with a switch: "Notify me when…".
struct NotifyToggle: View {
    let label: String
    @Binding var isOn: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        Toggle(isOn: $isOn) { Text(label).font(TextRole.body.font) }
            .toggleStyle(.switch).tint(tokens.primary)
            .padding(.horizontal, Space._6).padding(.vertical, Space._2)
    }
}
