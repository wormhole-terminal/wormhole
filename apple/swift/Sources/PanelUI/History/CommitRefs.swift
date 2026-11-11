// The branches, tags and HEAD that point at the commit, as chips (`style::status_badge`, `branch_chip`): HEAD
// in the primary on a tint of it, a branch with its mark in `git.branch` on `git.branch.background`, a tag in `git.tag`.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitRefs: View {
    let refs: [(kind: RefKind, name: String)]
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._1) {
            ForEach(Array(refs.enumerated()), id: \.offset) { _, ref in
                switch ref.kind {
                case .head: badge("HEAD", tokens.primary, tokens.primary.opacity(0.15))
                case .tag: badge(ref.name, tokens.gitTag, tokens.gitTag.opacity(0.15))
                case .branch:
                    HStack(spacing: Space._1) {
                        PanelIcon("arrow.triangle.branch", size: 12)
                        Text(ref.name).lineLimit(1)
                    }
                    .font(TextRole.caption.font.weight(.medium)).foregroundStyle(tokens.gitBranch)
                    .padding(.leading, Space._1).padding(.trailing, Space._2).frame(height: 20)
                    .background(tokens.gitBranchBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                }
            }
            Spacer(minLength: 0)
        }
    }

    private func badge(_ text: String, _ color: Color, _ fill: Color) -> some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(color)
            .padding(.horizontal, Space._2).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(fill, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }
}
