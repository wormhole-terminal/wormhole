// What the diff card shows for a submodule in place of lines (`features/git/diff/submodule.rs`): which commit to which in
// the code font, then "New commits" and "Commits left behind", each with its total and one two-line row per commit (the
// subject; the short hash, the author, how long ago), "and N more" after the ones listed, and core's sentences for why
// there is nothing to list or what else is in the folder. The body scrolls: a long list is never cut off by the card.

import DesignSystem
import Foundation
import PanelModel
import SwiftUI

struct SubmoduleCard: View {
    let commits: SubmoduleCommits
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    var body: some View {
        let now = Int(Date().timeIntervalSince1970)
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if let old = commits.old, let new = commits.new {
                    HStack(spacing: Space._2) {
                        Text(old)
                        PanelIcon("arrow.right", size: 12)
                        Text(new)
                    }
                    .font(codeFont.font(tokens.size(.caption))).foregroundStyle(tokens.textMuted)
                    .padding(.horizontal, Space._4).padding(.vertical, Space._3)
                }
                if let note = commits.nothing { self.note(note) }
                section(panelText("diff-submodule-new-commits", "New commits"), commits.ahead, total: commits.aheadTotal, now: now)
                section(panelText("diff-submodule-left-behind", "Commits left behind"), commits.behind, total: commits.behindTotal, now: now)
                if let note = commits.inside { self.note(note) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    /// A sentence in muted text.
    private func note(_ text: String) -> some View {
        Text(text).font(tokens.font(.body)).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._4).padding(.vertical, Space._2)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    /// A heading with how many commits there are, the commits listed, and how many more there are than listed.
    @ViewBuilder private func section(_ title: String, _ listed: [Commit], total: Int, now: Int) -> some View {
        if !listed.isEmpty {
            HStack(spacing: Space._2) {
                Text(title).textCase(.uppercase).font(tokens.font(.caption).weight(.semibold)).foregroundStyle(tokens.textMuted)
                BandCount(text: CountText.grouped(total))
            }
            .padding(.horizontal, Space._4).padding(.top, Space._3).padding(.bottom, Space._1)
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("\(title), \(CountText.grouped(total))")
            ForEach(listed) { commit in row(commit, now: now) }
            if total > listed.count {
                let more = CountText.grouped(total - listed.count)
                note(panelText("diff-submodule-more", ["count": .text(more)], "and \(more) more"))
            }
        }
    }

    /// One commit: its subject, and under it the short hash, the author and how long ago.
    private func row(_ commit: Commit, now: Int) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(commit.subject).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                .frame(height: PanelMetrics.nameLine)
            HStack(spacing: Space._2) {
                Text(commit.short).font(codeFont.font(tokens.size(.caption))).foregroundStyle(tokens.primary).fixedSize()
                Text(commit.author).lineLimit(1)
                Spacer(minLength: 0)
                Text(Age.text(seconds: now - commit.time)).monospacedDigit().fixedSize()
            }
            .font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
            .frame(height: PanelMetrics.metaLine)
        }
        .padding(.horizontal, Space._4).padding(.vertical, Space._1)
        .frame(maxWidth: .infinity, minHeight: Metric.rowTwoLine, alignment: .leading)
        .accessibilityElement(children: .combine)
    }
}
