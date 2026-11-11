// The branch's pull request: an icon in the colour of how it stands (a draft's muted), its number, and while
// it is open its checks (a green check once they pass, a red cross when one fails, a dashed circle while they
// run). A click opens it.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

struct PullRequestChip: View {
    let pr: PullRequestFact
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button { TitleClicks.openPullRequest(pr) } label: {
            HStack(spacing: TitleMetrics.extraGap) {
                ExtraChip(text: pr.label, symbol: ("arrow.triangle.pull", stateColour))
                if let checks = pr.openChecks {
                    let look = Self.checks(checks, tokens)
                    Image(systemName: look.symbol).font(.system(size: TitleMetrics.extraIcon - 2, weight: .medium)).foregroundStyle(look.colour)
                        .frame(width: TitleMetrics.extraIcon)
                }
            }
        }
        .buttonStyle(.plain)
        .help(pr.words).accessibilityLabel(pr.words).accessibilityAddTraits(.isLink)
    }

    private var stateColour: Color {
        switch pr.state {
        case .open: pr.draft ? tokens.textMuted : tokens.statusSuccessText
        case .merged: tokens.gitBranch
        case .closed: tokens.statusDangerText
        }
    }

    private static func checks(_ checks: PullRequestFact.Checks, _ tokens: ThemeTokens) -> (symbol: String, colour: Color) {
        switch checks {
        case .passing: ("checkmark.circle", tokens.statusSuccessText)
        case .failing: ("xmark.circle", tokens.statusDangerText)
        case .pending: ("circle.dashed", tokens.textMuted)
        }
    }
}
