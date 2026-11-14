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
                        LucideView(icon: .gitBranch, size: Metric.chevron + 3)
                            .frame(width: TitleMetrics.extraIcon)
                        if showsText { Text(branch ?? PanesText.detached).lineLimit(1).truncationMode(.tail) }
                    }
                    .font(tokens.font(.caption).weight(.medium))
                    .modifier(BranchChip(detached: detached, showsText: showsText, width: width))
                    .clipped()
                    .marked("branch.chip")
                    .contentTransition(.interpolate)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(PanesText.branch(branch ?? PanesText.detached))
                if let whose = plan.facts.superproject, plan.fit.words {
                    Text(whose).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
                        .lineLimit(1).fixedSize().padding(.leading, Space._2)
                        .help(PanesText.submodule(whose))
                }
            }
        }
    }
}

/// The branch chip's own colours and padding on the shared chip (the branch colour, or the warning one while detached).
private struct BranchChip: ViewModifier {
    let detached: Bool
    let showsText: Bool
    /// The width the title row planned for the chip: the wash fills all of it, as before the chip was shared.
    let width: CGFloat
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        Chip(
            fill: detached ? tokens.bannerWarningBackground : tokens.gitBranchBackground,
            text: detached ? tokens.statusWarningText : tokens.gitBranch, height: TitleMetrics.chipHeight, width: width,
            leading: Space._1,
            trailing: showsText ? Space._2 : Space._1
        ) { content }
    }
}
