// The branch chip: the branch glyph and its name on a 20 pt wash in the branch colour, cut to its glyph alone
// when the row is narrow; a detached HEAD says so in the warning colour. In a submodule, whose it is follows
// it while the row has room. A click opens the branch picker.

import DesignSystem
import PanesModel
import SwiftUI

struct BranchCrumb: View {
    let plan: TitlePlan
    let actions: TitleActions
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let branch = plan.facts.branch, let width = plan.fit.chip {
            let detached = branch == nil
            let showsText = width >= TitleMetrics.chipChrome + TitleMetrics.textMin / 2
            HStack(spacing: 0) {
                Button(action: actions.pickBranch) {
                    HStack(spacing: Space._1) {
                        Image(systemName: "arrow.triangle.branch").font(.system(size: Metric.chevron + 2))
                            .frame(width: TitleMetrics.extraIcon)
                        if showsText { Text(branch ?? PanesText.detached).lineLimit(1).truncationMode(.tail) }
                    }
                    .font(TextRole.caption.font.weight(.medium))
                    .foregroundStyle(detached ? tokens.statusWarningText : tokens.gitBranch)
                    .padding(.leading, Space._1).padding(.trailing, showsText ? Space._2 : Space._1)
                    .frame(width: width, height: TitleMetrics.chipHeight, alignment: .leading)
                    .background(detached ? tokens.bannerWarningBackground : tokens.gitBranchBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                    .clipped()
                    .contentTransition(.interpolate)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(PanesText.branch(branch ?? PanesText.detached))
                if let whose = plan.facts.superproject, plan.fit.words {
                    Text(whose).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                        .lineLimit(1).fixedSize().padding(.leading, Space._2)
                        .help(PanesText.submodule(whose))
                }
            }
        }
    }
}
