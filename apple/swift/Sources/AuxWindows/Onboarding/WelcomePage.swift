// The welcome page: the app's mark and name, what it is for, a few lines of the sidebar to show it, and the way in.
// It is what the window carries through the wormhole, so each part has its own moment to come in (`Entrance`).

import AuxModel
import DesignSystem
import SwiftUI

struct WelcomePage: View {
    let session: OnboardingSession
    let player: IntroPlayer
    @Environment(\.tokens) private var tokens

    var body: some View {
        let entrance = player.entrance
        let last = session.flow.isLast
        VStack(spacing: 0) {
            WelcomeLogo(progress: entrance.logo, player: player)
            Text(AuxText.brand).font(TextRole.title.font).padding(.top, Space._6).entering(entrance.title)
                .accessibilityAddTraits(.isHeader)
            Text(AuxText.tagline).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                .multilineTextAlignment(.center).padding(.top, Space._1).entering(entrance.tagline)
            VStack(spacing: Space._4) {
                WelcomeSpecimen()
                Text(AuxText.specimenCaption).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    .multilineTextAlignment(.center)
            }
            .padding(.top, Space._6 + Space._2).entering(entrance.body)
            VStack(spacing: Space._4) {
                HStack(spacing: Space._4) {
                    Button(last ? AuxText.backToWormhole : AuxText.getStarted) { session.send(last ? .start : .continue) }
                        .primaryAction().keyboardShortcut(.defaultAction)
                    Button(AuxText.skipIntro) { session.send(.skip) }.buttonStyle(.plain).foregroundStyle(tokens.textMuted)
                }
                HStack(spacing: Space._4) {
                    KeyHint(keys: "\u{23CE}", label: AuxText.hintContinue)
                    KeyHint(keys: "esc", label: AuxText.hintSkip)
                }
            }
            .padding(.top, Space._6 + Space._2).entering(entrance.actions)
        }
        .frame(width: OnboardingMetrics.welcomeWidth)
        .padding(.horizontal, Space._6)
    }
}
