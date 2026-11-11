// "How much to show" (`tailor_step.rs`): the Look step's walk through what the window displays, one question at a time,
// its answers drawn as the window each would give, the one in use ringed; Back and Start over, a summary, "Looks good".
// The walk is the core's and every answer is applied as it is given (`LookExtras`); this draws it.

import AuxModel
import DesignSystem
import SwiftUI

struct TailorStep: View {
    let look: LookExtras
    @Environment(\.tokens) private var tokens

    /// The strip under an answer's sketch, holding its number and name: two lines of a label (`STRIP_H`); room for the
    /// answer's line under the cards (`DETAIL_H`).
    private static let stripHeight: CGFloat = 48
    private static let detailHeight: CGFloat = 32

    var body: some View {
        if let walk = look.walk {
            VStack(alignment: .leading, spacing: Space._2) {
                SectionLabel(walk.heading, isCollapsed: nil)
                VStack(alignment: .leading, spacing: Space._3) { content(walk) }
                    .padding(Space._3)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
            }
            .accessibilityElement(children: .contain)
            .onChange(of: walk.announcement) { _, words in Announcements.post(words) }
        }
    }

    @ViewBuilder private func content(_ walk: TailorView) -> some View {
        switch walk.stage {
        case .asking: question(walk)
        case .summary: summary(walk)
        case .settled:
            HStack {
                Label(walk.settledText, systemImage: "checkmark").textRole(.body).foregroundStyle(tokens.textPrimary)
                    .symbolRenderingMode(.hierarchical)
                Spacer()
                link(walk.startOverLabel) { look.act(.startOver) }
            }
        }
    }

    private func question(_ walk: TailorView) -> some View {
        VStack(alignment: .leading, spacing: Space._3) {
            HStack(alignment: .firstTextBaseline) {
                Text(walk.question).textRole(.body, weight: .semibold)
                Spacer(minLength: Space._2)
                Text(walk.counter).textRole(.caption).foregroundStyle(tokens.textMuted)
            }
            HStack(alignment: .top, spacing: Space._2) {
                ForEach(Array(walk.answers.enumerated()), id: \.offset) { index, answer in card(answer, number: index + 1, index: index) }
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(walk.question)
            Text(walk.detail).textRole(.caption).foregroundStyle(tokens.textMuted)
                .frame(minHeight: Self.detailHeight, alignment: .topLeading)
            links(walk)
        }
    }

    private func card(_ answer: TailorAnswer, number: Int, index: Int) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        return Button { look.act(.answer(index)) } label: {
            VStack(alignment: .leading, spacing: 0) {
                WindowSketch(blocks: answer.sketch).padding(Space._2)
                HStack(alignment: .top, spacing: Space._1) {
                    KeyCap(text: "\(number)")
                    Text(answer.label).textRole(.caption, weight: answer.picked ? .semibold : .medium).frame(maxWidth: .infinity, alignment: .leading)
                }
                .padding(.horizontal, Space._2).padding(.bottom, Space._2)
                .frame(height: Self.stripHeight, alignment: .top)
            }
            .background(tokens.surfacePanel, in: shape)
            .overlay { shape.strokeBorder(answer.picked ? tokens.primary : tokens.border, lineWidth: answer.picked ? Metric.ring : tokens.metrics.borderWidth) }
            .contentShape(shape)
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
        .accessibilityLabel(answer.label)
        .accessibilityHint(answer.detail)
        .accessibilityAddTraits(answer.picked ? [.isButton, .isSelected] : .isButton)
    }

    private func summary(_ walk: TailorView) -> some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text(walk.summaryTitle).textRole(.body, weight: .semibold)
            VStack(alignment: .leading, spacing: Space._1) {
                ForEach(Array(walk.summary.enumerated()), id: \.offset) { _, line in
                    HStack(alignment: .firstTextBaseline, spacing: Space._3) {
                        Text(line.question).textRole(.caption).foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity, alignment: .leading)
                        Text(line.answer).textRole(.body, weight: .medium)
                    }
                }
            }
            HStack {
                links(walk)
                Spacer()
                Button(walk.looksGoodLabel) { look.act(.settle) }.primaryAction()
            }
        }
    }

    private func links(_ walk: TailorView) -> some View {
        HStack(spacing: Space._3) {
            if walk.canBack { link(walk.backLabel) { look.act(.back) } }
            link(walk.startOverLabel) { look.act(.startOver) }
        }
    }

    private func link(_ title: String, action: @escaping () -> Void) -> some View {
        Button(title, action: action).buttonStyle(.link).foregroundStyle(tokens.textLink).textRole(.caption)
    }
}
