// The branch's status after its chip: an amber dot while the tree has uncommitted changes, then the commits
// ahead of its upstream (↑) and behind it (↓) in muted digits; a count of none is left out. A click shows the
// Changes tab, where what changed is.

import DesignSystem
import PanesModel
import SwiftUI

struct BranchStatusMark: View {
    let status: BranchStatus
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            HStack(spacing: TitleMetrics.extraGap) {
                if status.dirty {
                    Circle().fill(tokens.statusWarning).frame(width: TitleMetrics.statusDot, height: TitleMetrics.statusDot)
                }
                if status.ahead > 0 { count("arrow.up", status.ahead) }
                if status.behind > 0 { count("arrow.down", status.behind) }
            }
            .font(TextRole.caption.font.weight(.medium))
            .foregroundStyle(tokens.textMuted)
            .frame(height: TitleMetrics.chipHeight)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(status.words)
        .accessibilityLabel(status.words)
    }

    private func count(_ symbol: String, _ n: Int) -> some View {
        HStack(spacing: 0) {
            Image(systemName: symbol).font(.system(size: TitleMetrics.extraIcon - 2, weight: .semibold)).frame(width: TitleMetrics.extraIcon)
            Text("\(n)").monospacedDigit()
        }
    }
}
