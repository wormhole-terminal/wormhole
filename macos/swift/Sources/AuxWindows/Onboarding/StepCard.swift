// The card a step sits on: "Step N of 3" and Skip setup on top, the title and a line under it, the step, and Back,
// the three dots and Continue (Start on the last step) along the bottom (`steps.rs`). It comes in as one piece.

import AuxModel
import DesignSystem
import SwiftUI

struct StepCard<Body: View>: View {
    let session: OnboardingSession
    let title: String
    let intro: String
    @ViewBuilder let content: Body
    @Environment(\.tokens) private var tokens

    var body: some View {
        let step = session.step
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: Space._2) {
                HStack {
                    Text(AuxText.stepCounter(step.number, of: OnboardingStep.counted, noAgents: step == .primer))
                        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    Spacer()
                    Button(AuxText.skipSetup) { session.send(.skip) }.buttonStyle(.plain)
                        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                }
                Text(title).font(TextRole.title.font).padding(.top, Space._2).accessibilityAddTraits(.isHeader)
                Text(intro).font(TextRole.body.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, Space._6).padding(.top, Space._6).padding(.bottom, Space._4)
            ScrollView { content.frame(maxWidth: .infinity, alignment: .leading) }.scrollBounceBehavior(.basedOnSize)
            StepFooter(session: session)
        }
        .frame(width: OnboardingMetrics.sheetWidth)
        .frame(minHeight: OnboardingMetrics.sheetMinHeight)
        .background(tokens.surfaceElevated)
        .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusLg).strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
        .shadow(color: tokens.shadow, radius: 24, y: 16)
        .accessibilityElement(children: .contain)
    }
}

private struct StepFooter: View {
    let session: OnboardingSession
    @Environment(\.tokens) private var tokens
    @Environment(\.stepKeys) private var keys

    var body: some View {
        let step = session.step
        VStack(spacing: 0) {
            Rule()
            HStack {
                Button { session.send(.back) } label: { Label(AuxText.back, systemImage: "chevron.left") }
                    .buttonStyle(.plain).frame(maxWidth: .infinity, alignment: .leading)
                HStack(spacing: Space._1) {
                    ForEach(1...OnboardingStep.counted, id: \.self) { n in
                        Capsule().fill(n == step.number ? tokens.primary : tokens.textFaint)
                            .frame(width: n == step.number ? OnboardingMetrics.dotBar : OnboardingMetrics.dot, height: OnboardingMetrics.dot)
                    }
                }
                .motion(Motion.selection, value: step.number)
                .accessibilityHidden(true)
                Button(step.isLast ? AuxText.start : AuxText.next) { session.send(step.isLast ? .start : .continue) }
                    .primaryAction().keyboardShortcut(keys ? KeyboardShortcut.defaultAction : nil)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(.horizontal, Space._6)
            .frame(height: OnboardingMetrics.footerHeight)
        }
    }
}
