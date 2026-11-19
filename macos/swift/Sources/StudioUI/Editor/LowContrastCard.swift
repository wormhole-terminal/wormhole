// The draft's pairs that fall short, in a warning card under the sections: each with its ratio, the colour it is
// on, a fix written as an expression (nothing changes until it is picked) and Show, which selects the text token
// (`low_contrast_card`). Save still works without.

import DesignSystem
import StudioModel
import SwiftUI

struct LowContrastCard: View {
    @Bindable var document: StudioDocument
    @Environment(\.tokens) private var tokens

    var body: some View {
        let issues = document.contrastIssues
        if !issues.isEmpty {
            VStack(alignment: .leading, spacing: Space._3) {
                Label(StudioWords.lowCount(issues.count), systemImage: "exclamationmark.triangle.fill")
                    .labelStyle(.titleAndIcon).font(TextRole.label.font).foregroundStyle(tokens.statusWarningText)
                    .symbolRenderingMode(.monochrome)
                ForEach(Array(issues.enumerated()), id: \.offset) { index, issue in
                    if index > 0 { Divider().overlay(tokens.border) }
                    VStack(alignment: .leading, spacing: Space._1) {
                        HStack {
                            Text(issue.check.fg).codeFont().lineLimit(1).truncationMode(.middle)
                            Spacer(minLength: 0)
                            RatioText(check: issue.check)
                        }
                        Text(StudioWords.contrastOn(issue.check.bg)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
                        if let fix = issue.fix {
                            actions(for: issue, fix: fix)
                        } else if let follows = issue.follows {
                            Text(StudioWords.contrastFollows(follows)).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                        }
                    }
                }
            }
            .padding(Space._4).frame(maxWidth: .infinity, alignment: .leading)
            .background(tokens.bannerWarningBackground, in: card)
            .overlay(card.strokeBorder(tokens.statusWarning, lineWidth: tokens.metrics.borderWidth))
            .padding(Space._4)
            .accessibilityElement(children: .contain)
        }
    }

    private var card: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radiusLg) }

    private func actions(for issue: ContrastIssue, fix: ContrastFix) -> some View {
        HStack(spacing: Space._2) {
            Button(fix.label) { document.set(fix.token, to: fix.expression) }
                .buttonStyle(.bordered).controlSize(.small).help(fix.expression)
            Button(StudioWords.show) { document.selected = issue.check.fg }
                .buttonStyle(.plain).font(TextRole.caption.font).foregroundStyle(tokens.textLink)
        }
    }
}
