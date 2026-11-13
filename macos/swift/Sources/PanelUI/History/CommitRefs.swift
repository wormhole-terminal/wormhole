// The branches, tags and HEAD that point at the commit, as chips (`style::status_badge`, `branch_chip`): HEAD
// in the primary on a tint of it, a branch with its mark in `git.branch` on `git.branch.background`, a tag in `git.tag`.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitRefs: View {
    let refs: [(kind: RefKind, name: String)]
    /// "+3" after the refs shown, when the commit has more.
    var more: String?
    @Environment(\.tokens) private var tokens

    var body: some View {
        // Onto more lines in a narrow panel, each name whole.
        WrapLayout(spacing: Space._1) {
            ForEach(Array(refs.enumerated()), id: \.offset) { _, ref in
                switch ref.kind {
                case .head: StatusBadge(text: "HEAD", color: tokens.primary) // text: HEAD is git's name for the current commit
                case .tag: StatusBadge(text: ref.name, color: tokens.gitTag)
                case .branch:
                    HStack(spacing: Space._1) {
                        PanelIcon("arrow.triangle.branch", size: 12)
                        Text(ref.name).lineLimit(1).fixedSize()
                    }
                    .font(tokens.font(.caption).weight(.medium)).foregroundStyle(tokens.gitBranch)
                    .padding(.leading, Space._1).padding(.trailing, Space._2).frame(height: 20)
                    .background(tokens.gitBranchBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                }
            }
            if let more { Text(more).foregroundStyle(tokens.textMuted) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
